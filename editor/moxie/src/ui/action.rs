//! Inspects whatever the timeline has selected: an action's own
//! properties, or a block's.
//!
//! The reflect inspector cannot reach these - it addresses one
//! component of one entity, and an action is scene data. So this
//! edits the [`EditorScene`] directly, by the path the timeline
//! selected the node with.

use core::time::Duration;

use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use bevy::reflect::PartialReflect;
use bevy_motiongfx::scene::backend::{AnimEase, AnimInterp, Backend};
use bevy_motiongfx::scene::id::SceneUid;
use fynix_mock::composer::Composer;
use fynix_mock::elem;
use fynix_mock::ui::ElementHandle;
use motiongfx_scene::block::{ActionCmd, Block, Combinator, Node};
use moxie_ui::elements::{Frame, Label, Panel};
use moxie_ui::inspector::{Source, inspect_value};
use moxie_ui::reactive::{BevyHost, BevyUi, value_changed};
use moxie_ui::theme::EditorTheme;

use super::PANEL_PADDING;
use crate::{EditorScene, SelectedAction};

/// The action panel, as kernel nodes.
pub(super) struct ActionPanel;

impl Composer<BevyHost> for ActionPanel {
    type Element = Panel;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Panel> {
        ui.elem(elem!(
            Panel,
            direction = FlexDirection::Column,
            row_gap = px(8),
            padding = px(PANEL_PADDING),
            scrolls = true
        ))
        // The shape only: each input binds its own value, so typing
        // into one never rebuilds the panel out from under it.
        .watch(value_changed(shape), build)
        .handle()
    }
}

/// One number in the selected node that an input writes back.
///
/// Named rather than captured, because an input re-reads and rewrites
/// it long after the panel was built.
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
/// absent - they move without rebuilding anything.
#[derive(PartialEq)]
struct Shape {
    path: Option<Vec<usize>>,
    /// Empty when the path no longer lands on a node.
    kind: &'static str,
    rows: Vec<(String, String)>,
    edits: Vec<(String, Edit)>,
    /// The action's target value. Which widget draws it is the
    /// registry's business, not this panel's.
    value: Option<Pooled>,
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
            rows: Vec::new(),
            edits: Vec::new(),
            value: None,
        })
}

fn build(ui: &mut BevyUi) {
    let theme = ui.world.resource::<EditorTheme>().clone();
    let shape = shape(ui.world, ui.parent());

    let Some(path) = shape.path else {
        note(ui, &theme, "Nothing selected");
        return;
    };
    if shape.kind.is_empty() {
        note(ui, &theme, "Selection is no longer in the scene");
        return;
    }

    heading(ui, &theme, shape.kind);
    for (name, value) in shape.rows {
        row(ui, &theme, &name, &value);
    }
    for (name, edit) in shape.edits {
        let source = Property {
            path: path.clone(),
            edit,
        };
        line(ui, name, theme.text_muted, move |ui| {
            inspect_value(ui, &source);
        });
    }

    // Whatever the registry has for the type it turns out to hold.
    if let Some(pooled) = shape.value {
        line(ui, "Value".to_string(), theme.text_muted, move |ui| {
            inspect_value(ui, &pooled);
        });
    }
}

/// The selected node, as what to show and what can be changed.
fn summarize(world: &World, path: &[usize]) -> Option<Shape> {
    let scene = world.get_resource::<EditorScene>()?.scene();
    let node = node_at(&scene.0.animation, path)?;

    let (delay, action) = match node {
        Node::Block { block, delay } => {
            let mut edits = Vec::new();
            if matches!(block.combinator, Combinator::Flow(_)) {
                edits.push(("Stagger".to_string(), Edit::Stagger));
            }
            if delay.is_some() {
                edits.push(("Delay".to_string(), Edit::Delay));
            }

            return Some(Shape {
                path: Some(path.to_vec()),
                kind: "Block",
                value: None,
                rows: vec![
                    (
                        "Combinator".into(),
                        combinator_name(&block.combinator),
                    ),
                    (
                        "Children".into(),
                        block.children.len().to_string(),
                    ),
                ],
                edits,
            });
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
        value: Some(Pooled(action.value)),
        rows: vec![
            ("Subject".into(), subject_name(action)),
            ("Type".into(), action.field.type_name().to_string()),
            ("Field".into(), action.field.path().to_string()),
            ("Operation".into(), format!("{:?}", action.op)),
        ],
        edits,
    })
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
        let mut seen: Option<Option<Box<dyn PartialReflect>>> = None;

        Box::new(move |world| {
            let current = pooled.get(world);
            let fired = !seen.as_ref().is_some_and(|last| {
                match (last, &current) {
                    (Some(last), Some(current)) => last
                        .reflect_partial_eq(&**current)
                        .unwrap_or(false),
                    (None, None) => true,
                    _ => false,
                }
            });
            seen = Some(current);
            fired
        })
    }

    fn boxed(&self) -> Box<dyn Source> {
        Box::new(*self)
    }
}

/// One of the node's own properties, reflected.
impl Source for Property {
    fn get(&self, world: &World) -> Option<Box<dyn PartialReflect>> {
        let scene = world.get_resource::<EditorScene>()?.scene();
        let node = node_at(&scene.0.animation, &self.path)?;

        match (self.edit, node) {
            // `None` is the default curve, which is linear - so the
            // picker shows that rather than a fourth "unset" state.
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
        let mut seen: Option<Option<Box<dyn PartialReflect>>> = None;

        Box::new(move |world| {
            let current = property.get(world);
            let fired = !seen.as_ref().is_some_and(|last| {
                match (last, &current) {
                    (Some(last), Some(current)) => last
                        .reflect_partial_eq(&**current)
                        .unwrap_or(false),
                    (None, None) => true,
                    _ => false,
                }
            });
            seen = Some(current);
            fired
        })
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
            match block.combinator {
                Combinator::Flow(delay) => Some(delay.as_secs_f32()),
                _ => None,
            }
        }
        (Edit::Duration, Node::Action { action, .. }) => {
            Some(action.duration.as_secs_f32())
        }
        _ => None,
    }
}

/// Writes one of the properties measured in seconds.
fn set_seconds(node: &mut Node<Backend>, edit: Edit, value: f32) {
    // Never negative: an action cannot run backwards.
    let seconds = Duration::from_secs_f32(value.max(0.0));

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

fn subject_name(action: &ActionCmd<Backend>) -> String {
    let SceneUid::Entity(uid) = action.subject;
    format!("{uid}")
}

fn combinator_name(combinator: &Combinator) -> String {
    match combinator {
        Combinator::Chain => "Chain".to_string(),
        Combinator::All => "All".to_string(),
        Combinator::Any => "Any".to_string(),
        // Its stagger is editable, so the row above only names it.
        Combinator::Flow(_) => "Flow".to_string(),
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

/// One `name: value` line, for what this panel cannot change.
fn row(
    ui: &mut BevyUi,
    theme: &EditorTheme,
    name: &str,
    value: &str,
) {
    let value = value.to_string();
    let primary = theme.text_primary;

    line(ui, name.to_string(), theme.text_muted, move |ui| {
        ui.elem(elem!(
            Label,
            text = value,
            color = Some(primary),
            wrap = false
        ));
    });
}

/// The name on the left, whatever it takes on the right.
fn line(
    ui: &mut BevyUi,
    name: String,
    muted: Color,
    body: impl FnOnce(&mut BevyUi),
) {
    ui.elem(elem!(
        Frame,
        width = percent(100),
        direction = FlexDirection::Row,
        justify = JustifyContent::SpaceBetween,
        align = AlignItems::Center,
        column_gap = px(8),
        padding = UiRect::vertical(px(3))
    ))
    .with(move |ui| {
        ui.elem(elem!(Label, text = name, color = Some(muted)));
        body(ui);
    });
}

/// What the panel says when there is nothing to show.
fn note(ui: &mut BevyUi, theme: &EditorTheme, text: &str) {
    ui.elem(elem!(
        Label,
        text = text.to_string(),
        color = Some(theme.text_muted)
    ));
}
