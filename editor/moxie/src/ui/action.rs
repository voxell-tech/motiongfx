//! Inspects whatever the timeline has selected: an action's own
//! properties, or a block's.
//!
//! The reflect inspector cannot reach these: it addresses one
//! component of one entity, and an action is scene data. This edits
//! the [`EditorScene`] directly, by the path the timeline selected
//! the node with.

use core::time::Duration;

use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use bevy::reflect::PartialReflect;
use bevy_motiongfx::scene::backend::{AnimEase, AnimInterp, Backend};
use bevy_motiongfx::scene::id::{SceneUid, SceneUidMap};
use fynix_mock::composer::Composer;
use fynix_mock::elem;
use fynix_mock::ui::ElementHandle;
use motiongfx_scene::block::{ActionCmd, Block, Combinator, Node};
use motiongfx_scene::refs::FieldRef;
use moxie_ui::elements::{Frame, Label, ScrollArea, display_name};
use moxie_ui::inspector::{
    FieldRow, Source, inspect_value, reflect_changed,
};
use moxie_ui::reactive::{BevyHost, BevyUi, value_changed};
use moxie_ui::theme::EditorTheme;

use super::{PANEL_PADDING, hierarchy};
use crate::{EditorScene, SelectedAction};

/// The action panel, as kernel nodes.
pub(super) struct ActionPanel;

impl Composer<BevyHost> for ActionPanel {
    type Element = ScrollArea;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, ScrollArea> {
        ui.elem(elem!(
            ScrollArea,
            flex_grow = 1.0f32,
            row_gap = px(8),
            padding = px(PANEL_PADDING),
            scroll_x = false
        ))
        // The shape only: each input binds its own value, so typing
        // into one never rebuilds the panel out from under it.
        .watch(value_changed(shape), build)
        .handle()
    }
}

/// One number in the selected node that an input writes back.
///
/// Named, not captured: an input re-reads and rewrites it long after
/// the panel was built.
#[derive(Clone, Copy, PartialEq)]
enum Edit {
    /// How long the action runs for.
    Duration,
    /// The node's own offset from where its parent starts it.
    Delay,
    /// How far apart a `Flow` block staggers its children.
    Stagger,
    /// The curve the action follows.
    Ease,
    /// How its values are blended.
    Interp,
}

/// Everything a rebuild depends on. The numbers are deliberately
/// absent: they move without rebuilding anything.
#[derive(PartialEq)]
struct Shape {
    path: Option<Vec<usize>>,
    /// Empty when the path no longer lands on a node.
    kind: &'static str,
    /// Set only for an action: a block has none to show.
    subject: Option<Subject>,
    rows: Vec<(String, String)>,
    edits: Vec<(String, Edit)>,
    /// The action's target value. Which widget draws it is the
    /// registry's business, not this panel's.
    value: Option<Pooled>,
}

/// An action's subject, split so its id can render muted where its
/// name cannot.
#[derive(Clone, PartialEq)]
struct Subject {
    /// Its [`Name`], if it still has an entity.
    name: Option<String>,
    /// The head of its id - a whole uuid is unreadable, but two
    /// entities sharing a name still need telling apart.
    head: String,
}

/// The action's target value, wherever the pool keeps it.
#[derive(Clone, Copy, PartialEq)]
struct Pooled(Uuid);

/// One property of the selected node, as somewhere a widget can read
/// and write. Which widget that is follows from the type it hands
/// back, so this never names one.
#[derive(Clone)]
struct Property {
    path: Vec<usize>,
    edit: Edit,
}

fn shape(world: &World, _: Entity) -> Shape {
    let path = world
        .get_resource::<SelectedAction>()
        .and_then(|selected| selected.0.clone());

    path.as_ref()
        .and_then(|path| summarize(world, path))
        .unwrap_or(Shape {
            path,
            kind: "",
            subject: None,
            rows: Vec::new(),
            edits: Vec::new(),
            value: None,
        })
}

fn build(ui: &mut BevyUi) {
    let theme = ui.theme;
    let shape = shape(ui.world, ui.parent());

    let Some(path) = shape.path else {
        note(ui, "Nothing selected");
        return;
    };
    if shape.kind.is_empty() {
        note(ui, "Selection is no longer in the scene");
        return;
    }

    heading(ui, theme, shape.kind);
    if let Some(subject) = shape.subject {
        let primary = theme.text_primary;
        let muted = theme.text_muted;
        ui.compose(FieldRow {
            label: "Subject".to_string(),
            color: muted,
            bold: false,
            depth: 0,
            value: move |ui: &mut BevyUi| {
                ui.elem(elem!(
                    Frame,
                    direction = FlexDirection::Row,
                    align = AlignItems::Center,
                    column_gap = px(4)
                ))
                .with(move |ui| {
                    if let Some(name) = subject.name {
                        ui.elem(elem!(
                            Label,
                            text = name,
                            color = Some(primary),
                            wrap = false
                        ));
                    }
                    ui.elem(elem!(
                        Label,
                        text = format!("#{}", subject.head),
                        color = Some(muted),
                        wrap = false
                    ));
                });
            },
        });
    }
    for (name, value) in shape.rows {
        let primary = theme.text_primary;
        ui.compose(FieldRow {
            label: name,
            color: theme.text_muted,
            bold: false,
            depth: 0,
            value: move |ui: &mut BevyUi| {
                ui.elem(elem!(
                    Label,
                    text = value,
                    color = Some(primary),
                    wrap = false
                ));
            },
        });
    }
    for (name, edit) in shape.edits {
        let source = Property {
            path: path.clone(),
            edit,
        };
        ui.compose(FieldRow {
            label: name,
            color: theme.text_muted,
            bold: false,
            depth: 0,
            value: move |ui: &mut BevyUi| inspect_value(ui, &source),
        });
    }

    // Whatever the registry has for the type it turns out to hold.
    if let Some(pooled) = shape.value {
        ui.compose(FieldRow {
            label: "Value".to_string(),
            color: theme.text_muted,
            bold: false,
            depth: 0,
            value: move |ui: &mut BevyUi| inspect_value(ui, &pooled),
        });
    }
}

/// The selected node, as what to show and what can be changed.
///
/// An empty `path` names the tree's own root - a block like any
/// other, just never wrapped in a [`Node`] of its own, so it has no
/// delay to show or edit.
fn summarize(world: &World, path: &[usize]) -> Option<Shape> {
    let scene = world.get_resource::<EditorScene>()?.scene();

    if path.is_empty() {
        return Some(block_shape(
            Vec::new(),
            &scene.0.animation,
            None,
        ));
    }

    let node = node_at(&scene.0.animation, path)?;
    let (delay, action) = match node {
        Node::Block { block, delay } => {
            return Some(block_shape(path.to_vec(), block, *delay));
        }
        Node::Action { action, delay } => (delay, action),
    };

    let mut edits = vec![("Duration".to_string(), Edit::Duration)];
    if delay.is_some() {
        edits.push(("Delay".to_string(), Edit::Delay));
    }
    edits.push(("Ease".to_string(), Edit::Ease));
    edits.push(("Interpolation".to_string(), Edit::Interp));

    Some(Shape {
        path: Some(path.to_vec()),
        kind: "Action",
        subject: Some(subject_of(world, action)),
        value: Some(Pooled(action.value)),
        rows: vec![
            ("Field".into(), field_name(world, &action.field)),
            ("Operation".into(), format!("{:?}", action.op)),
        ],
        edits,
    })
}

/// A block's own row of info: its combinator and child count, plus
/// whatever of its timing the [`Node`] wrapping it (if any) lets
/// through.
fn block_shape(
    path: Vec<usize>,
    block: &Block<Backend>,
    delay: Option<Duration>,
) -> Shape {
    let mut edits = Vec::new();
    if matches!(block.combinator, Combinator::Flow(_)) {
        edits.push(("Stagger".to_string(), Edit::Stagger));
    }
    if delay.is_some() {
        edits.push(("Delay".to_string(), Edit::Delay));
    }

    Shape {
        path: Some(path),
        kind: "Block",
        subject: None,
        value: None,
        rows: vec![
            (
                "Combinator".into(),
                combinator_name(&block.combinator).to_string(),
            ),
            ("Children".into(), block.children.len().to_string()),
        ],
        edits,
    }
}

/// The node `path` names.
fn node_at<'a>(
    root: &'a Block<Backend>,
    path: &[usize],
) -> Option<&'a Node<Backend>> {
    let (&first, rest) = path.split_first()?;

    let mut node = root.children.get(first)?;
    for &index in rest {
        let Node::Block { block, .. } = node else {
            return None;
        };
        node = block.children.get(index)?;
    }
    Some(node)
}

/// The same walk, to change what it lands on.
fn node_at_mut<'a>(
    root: &'a mut Block<Backend>,
    path: &[usize],
) -> Option<&'a mut Node<Backend>> {
    let (&first, rest) = path.split_first()?;

    let mut node = root.children.get_mut(first)?;
    for &index in rest {
        let Node::Block { block, .. } = node else {
            return None;
        };
        node = block.children.get_mut(index)?;
    }
    Some(node)
}

/// The pooled value, whatever column it lives in.
///
/// The type comes back with the value, so nothing here has to name
/// which of them it is.
impl Source for Pooled {
    fn get(&self, world: &World) -> Option<Box<dyn PartialReflect>> {
        let values =
            &world.get_resource::<EditorScene>()?.scene().0.values;

        values
            .f32
            .get(&self.0)
            .map(|value| Box::new(*value) as Box<dyn PartialReflect>)
            .or_else(|| {
                values.vec3.get(&self.0).map(|value| {
                    Box::new(*value) as Box<dyn PartialReflect>
                })
            })
            .or_else(|| {
                values.quat.get(&self.0).map(|value| {
                    Box::new(*value) as Box<dyn PartialReflect>
                })
            })
    }

    fn set(&self, world: &mut World, value: &dyn PartialReflect) {
        let Some(mut editor) =
            world.get_resource_mut::<EditorScene>()
        else {
            return;
        };
        let values = &mut editor.edit().0.values;

        if let Some(slot) = values.f32.get_mut(&self.0) {
            let _ = slot.try_apply(value);
        } else if let Some(slot) = values.vec3.get_mut(&self.0) {
            let _ = slot.try_apply(value);
        } else if let Some(slot) = values.quat.get_mut(&self.0) {
            let _ = slot.try_apply(value);
        }
    }

    fn changed(
        &self,
    ) -> Box<dyn FnMut(&World) -> bool + Send + Sync> {
        let pooled = *self;
        Box::new(reflect_changed(move |world| pooled.get(world)))
    }

    fn boxed(&self) -> Box<dyn Source> {
        Box::new(*self)
    }
}

/// One of the node's own properties, reflected.
impl Source for Property {
    fn get(&self, world: &World) -> Option<Box<dyn PartialReflect>> {
        let scene = world.get_resource::<EditorScene>()?.scene();

        // The root has no `Node` of its own, so no `Ease`, `Interp`,
        // `Duration` or `Delay` either - only ever reached for its
        // own `Stagger`.
        if self.path.is_empty() {
            return Some(Box::new(stagger_seconds(&scene.0.animation)?));
        }
        let node = node_at(&scene.0.animation, &self.path)?;

        match (self.edit, node) {
            // `None` is the default curve, linear, so the picker
            // shows that instead of a fourth "unset" state.
            (Edit::Ease, Node::Action { action, .. }) => Some(
                Box::new(action.ease.unwrap_or(AnimEase::Linear)),
            ),
            (Edit::Interp, Node::Action { action, .. }) => Some(
                Box::new(action.interp.unwrap_or(AnimInterp::Linear)),
            ),
            _ => Some(Box::new(seconds(node, self.edit)?)),
        }
    }

    fn set(&self, world: &mut World, value: &dyn PartialReflect) {
        let Some(mut editor) =
            world.get_resource_mut::<EditorScene>()
        else {
            return;
        };
        let scene = editor.edit();

        if self.path.is_empty() {
            if let Some(value) = f32::from_reflect(value) {
                scene.0.animation.combinator =
                    Combinator::Flow(clamp_seconds(value));
            }
            return;
        }
        let Some(node) =
            node_at_mut(&mut scene.0.animation, &self.path)
        else {
            return;
        };

        match (self.edit, node) {
            (Edit::Ease, Node::Action { action, .. }) => {
                action.ease = AnimEase::from_reflect(value);
            }
            (Edit::Interp, Node::Action { action, .. }) => {
                action.interp = AnimInterp::from_reflect(value);
            }
            (edit, node) => {
                if let Some(value) = f32::from_reflect(value) {
                    set_seconds(node, edit, value);
                }
            }
        }
    }

    fn changed(
        &self,
    ) -> Box<dyn FnMut(&World) -> bool + Send + Sync> {
        let property = self.clone();
        Box::new(reflect_changed(move |world| property.get(world)))
    }

    fn boxed(&self) -> Box<dyn Source> {
        Box::new(self.clone())
    }
}

/// What `edit` reads as, for the properties measured in seconds.
fn seconds(node: &Node<Backend>, edit: Edit) -> Option<f32> {
    match (edit, node) {
        (Edit::Delay, Node::Block { delay, .. })
        | (Edit::Delay, Node::Action { delay, .. }) => {
            Some(delay.unwrap_or_default().as_secs_f32())
        }
        (Edit::Stagger, Node::Block { block, .. }) => {
            stagger_seconds(block)
        }
        (Edit::Duration, Node::Action { action, .. }) => {
            Some(action.duration.as_secs_f32())
        }
        _ => None,
    }
}

/// A `Flow` block's own stagger, in seconds - `None` for any other
/// combinator.
fn stagger_seconds(block: &Block<Backend>) -> Option<f32> {
    match block.combinator {
        Combinator::Flow(delay) => Some(delay.as_secs_f32()),
        _ => None,
    }
}

/// Never negative: nothing here runs backwards.
fn clamp_seconds(value: f32) -> Duration {
    Duration::from_secs_f32(value.max(0.0))
}

/// Writes one of the properties measured in seconds.
fn set_seconds(node: &mut Node<Backend>, edit: Edit, value: f32) {
    let seconds = clamp_seconds(value);

    match (edit, node) {
        (Edit::Delay, Node::Block { delay, .. })
        | (Edit::Delay, Node::Action { delay, .. }) => {
            *delay = Some(seconds);
        }
        (Edit::Stagger, Node::Block { block, .. }) => {
            block.combinator = Combinator::Flow(seconds);
        }
        (Edit::Duration, Node::Action { action, .. }) => {
            action.duration = seconds;
        }
        _ => {}
    }
}

/// The action's subject, as its own [`Name`] (if it still has an
/// entity) and the head of its id.
fn subject_of(world: &World, action: &ActionCmd<Backend>) -> Subject {
    let SceneUid::Entity(uid) = action.subject;

    let name = world
        .get_resource::<SceneUidMap>()
        .and_then(|map| map.entity(uid))
        .and_then(|entity| world.get::<Name>(entity))
        .map(|name| name.as_str().to_string());

    Subject {
        name,
        head: hierarchy::uid_head(uid),
    }
}

/// The type's own display name, the same [`ReflectInspectable`]-aware
/// one the entity inspector shows it by, followed by the path to the
/// field this action drives - `Transform::translation::x`. Falls back
/// to the type path's own last segment when the registry has never
/// heard of it.
///
/// [`ReflectInspectable`]: moxie_ui::inspector::ReflectInspectable
fn field_name(world: &World, field: &FieldRef) -> String {
    let type_name = field.type_name().to_string();
    let registry = world.resource::<AppTypeRegistry>().read();

    let name = registry
        .get_with_type_path(&type_name)
        .map(display_name)
        .unwrap_or_else(|| {
            type_name
                .rsplit("::")
                .next()
                .unwrap_or(&type_name)
                .to_string()
                .into()
        });

    format!("{name}{}", field.path())
}

fn combinator_name(combinator: &Combinator) -> &'static str {
    match combinator {
        Combinator::Chain => "Chain",
        Combinator::All => "All",
        Combinator::Any => "Any",
        // Its stagger is editable, so the row above only names it.
        Combinator::Flow(_) => "Flow",
    }
}

/// A section header, which is all this panel has by way of structure.
fn heading(ui: &mut BevyUi, theme: &EditorTheme, text: &str) {
    ui.elem(elem!(
        Label,
        text = text.to_string(),
        bold = true,
        color = Some(theme.text_primary)
    ));
}

/// What the panel says when there is nothing to show.
fn note(ui: &mut BevyUi, text: &str) {
    ui.elem(elem!(
        Label,
        text = text.to_string(),
        color = Some(ui.theme.text_muted)
    ));
}
