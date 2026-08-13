//! Walks a target's reflected value into a collapsible hierarchy and
//! renders it.
//!
//! A struct with no [`ReflectInspect`] of its own is not a leaf: its
//! fields become a group, shown under a header that folds them away.
//! Only a registered type stops the walk and becomes an editable row.

use std::any::TypeId;
use std::collections::HashSet;
use std::sync::Arc;

use bevy::ecs::change_detection::Tick;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::reflect::{PartialReflect, ReflectRef, TypeRegistry};
use bevy::ui_widgets::Activate;

use bevy_fynix::ElementMutExt;
use fynix_mock::{elem, val};

use super::{Field, InspectorTarget, ReflectInspect};
use crate::fynix::{
    ButtonElemCursor, Frame, FrameCursor, Icon, IconCursor, Label,
    TintButton,
};
use crate::icons;
use crate::reactive::BevyUi;
use crate::theme::EditorTheme;

/// The fold chevron's own rotation, clockwise from the asset's
/// resting up-pointing orientation: right when a group is folded
/// shut, down when its children show.
const CHEVRON_FOLDED: f32 = 90.0;
const CHEVRON_OPEN: f32 = 180.0;

/// One row the walk found: a leaf with a widget, or a struct grouped
/// under a collapsible header because it has none of its own.
#[derive(Clone, PartialEq)]
enum Entry {
    Leaf {
        path: String,
        type_id: TypeId,
    },
    Group {
        path: String,
        name: String,
        children: Vec<Entry>,
    },
}

/// One field: a leaf if a widget is registered for its type, a
/// collapsible group if it is a struct with none of its own, or
/// dropped if it's neither.
fn push_entry(
    registry: &TypeRegistry,
    value: &dyn PartialReflect,
    path: &str,
    name: &str,
    out: &mut Vec<Entry>,
) {
    if let Some(type_id) =
        value.get_represented_type_info().map(|i| i.type_id())
        && registry.get_type_data::<ReflectInspect>(type_id).is_some()
    {
        out.push(Entry::Leaf {
            path: path.to_string(),
            type_id,
        });
        return;
    }

    if matches!(
        value.reflect_ref(),
        ReflectRef::Struct(_) | ReflectRef::TupleStruct(_)
    ) {
        let children = collect_entries(registry, value, path);
        // An empty struct has nothing to fold away, so it isn't worth
        // a header of its own.
        if !children.is_empty() {
            out.push(Entry::Group {
                path: path.to_string(),
                name: name.to_string(),
                children,
            });
        }
    }
}

/// The entries for `value`'s own fields, one level down from `prefix`.
fn collect_entries(
    registry: &TypeRegistry,
    value: &dyn PartialReflect,
    prefix: &str,
) -> Vec<Entry> {
    let mut out = Vec::new();
    match value.reflect_ref() {
        ReflectRef::Struct(value) => {
            for i in 0..value.field_len() {
                let (Some(name), Some(field)) =
                    (value.name_at(i), value.field_at(i))
                else {
                    continue;
                };
                push_entry(
                    registry,
                    field,
                    &join(prefix, name),
                    name,
                    &mut out,
                );
            }
        }
        ReflectRef::TupleStruct(value) => {
            for i in 0..value.field_len() {
                let Some(field) = value.field(i) else {
                    continue;
                };
                let index = i.to_string();
                push_entry(
                    registry,
                    field,
                    &join(prefix, &index),
                    &index,
                    &mut out,
                );
            }
        }
        _ => {}
    }
    out
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}.{name}")
    }
}

/// The last path segment, which is what a row labels itself with -
/// the group above it already said the rest.
fn leaf_name(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

/// `target`'s entries, in walk order.
///
/// Unlike a nested field, the root itself is never wrapped in a
/// group: if it is a leaf type it is the one row shown, and otherwise
/// its fields are listed directly rather than folded under a header
/// for a group that was never named.
fn entries(world: &World, target: InspectorTarget) -> Vec<Entry> {
    let mut out = Vec::new();
    target.read(world, |value| {
        // Taken inside the read, where `InspectorTarget` has already
        // released its own guard.
        let registry = world.resource::<AppTypeRegistry>().read();
        let value = value.as_partial_reflect();

        if let Some(type_id) =
            value.get_represented_type_info().map(|i| i.type_id())
            && registry
                .get_type_data::<ReflectInspect>(type_id)
                .is_some()
        {
            out.push(Entry::Leaf {
                path: String::new(),
                type_id,
            });
        } else {
            out = collect_entries(&registry, value, "");
        }
    });
    out
}

/// Fires when the *shape* of `target` changes, meaning its set of
/// entries, and not merely their values.
///
/// Values ride on bindings rather than rebuilds, which is what lets a
/// number input keep focus while the value changes underneath it: a
/// rebuild would despawn the widget mid-edit. The tick is checked
/// first so the reflection walk only runs when something actually
/// touched the target.
fn shape_changed(
    target: InspectorTarget,
) -> impl FnMut(&World, Entity) -> bool {
    let mut seen_tick: Option<Tick> = None;
    let mut seen_shape: Option<Vec<Entry>> = None;
    move |world, _| {
        let tick = target.changed_tick(world);
        if seen_shape.is_some() && tick == seen_tick {
            return false;
        }
        seen_tick = tick;

        let current = entries(world, target);
        let fired = seen_shape.as_ref() != Some(&current);
        seen_shape = Some(current);
        fired
    }
}

/// Which groups are folded shut, keyed by the target they belong to
/// so two inspectors never share state, and by the path to the
/// group's own field within it.
///
/// This is UI state, not part of the reflected value, so it lives
/// beside the tree rather than on it: folding a group must not read
/// as the target's shape changing and trigger [`shape_changed`].
#[derive(Resource, Default)]
struct FoldedGroups(HashSet<(InspectorTarget, Arc<str>)>);

fn is_folded(
    world: &World,
    target: InspectorTarget,
    path: &str,
) -> bool {
    world.get_resource::<FoldedGroups>().is_some_and(|folded| {
        folded.0.contains(&(target, path.into()))
    })
}

fn toggle_folded(
    world: &mut World,
    target: InspectorTarget,
    path: Arc<str>,
) {
    let mut folded =
        world.get_resource_or_insert_with(FoldedGroups::default);
    if !folded.0.remove(&(target, path.clone())) {
        folded.0.insert((target, path));
    }
}

/// Fires when `path`'s folded state under `target` flips, and on the
/// first poll so a binding starts out in sync.
fn fold_changed(
    target: InspectorTarget,
    path: Arc<str>,
) -> impl FnMut(&World, Entity) -> bool {
    let mut seen: Option<bool> = None;
    move |world, _| {
        let current = is_folded(world, target, &path);
        let fired = seen != Some(current);
        seen = Some(current);
        fired
    }
}

/// Editable rows for everything reflectable under `target`, as kernel
/// nodes.
pub fn inspector_fields(ui: &mut BevyUi, target: InspectorTarget) {
    ui.elem(elem!(
        Frame,
        width = percent(100),
        direction = FlexDirection::Column,
        row_gap = px(4)
    ))
    .insert(TabGroup::new(0))
    .watch(shape_changed(target), move |ui| {
        build_entries(ui, target, entries(ui.world, target));
    });
}

fn build_entries(
    ui: &mut BevyUi,
    target: InspectorTarget,
    entries: Vec<Entry>,
) {
    for entry in entries {
        match entry {
            Entry::Leaf { path, type_id } => {
                build_leaf(ui, target, path, type_id)
            }
            Entry::Group {
                path,
                name,
                children,
            } => build_group(ui, target, path, name, children),
        }
    }
}

fn build_leaf(
    ui: &mut BevyUi,
    target: InspectorTarget,
    path: String,
    type_id: TypeId,
) {
    let drawer = {
        let registry = ui.world.resource::<AppTypeRegistry>().read();
        registry.get_type_data::<ReflectInspect>(type_id).cloned()
    };
    let Some(drawer) = drawer else { return };

    // Dimmer than the value it labels, the way most engine
    // inspectors read: the field name is a caption, not the content.
    let muted = ui.world.resource::<EditorTheme>().text_muted;
    let label = leaf_name(&path).to_string();
    let field = Field::new(target, path);
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
        ui.elem(elem!(Label, text = label, color = Some(muted)));
        drawer.build(&field, ui);
    });
}

fn build_group(
    ui: &mut BevyUi,
    target: InspectorTarget,
    path: String,
    name: String,
    children: Vec<Entry>,
) {
    let path: Arc<str> = path.into();
    let theme = ui.world.resource::<EditorTheme>().clone();

    ui.elem(elem!(
        Frame,
        width = percent(100),
        direction = FlexDirection::Column,
        row_gap = px(2)
    ))
    .with(move |ui| {
        let click_path = path.clone();
        let glyph_path = path.clone();
        ui.elem(elem!(
            !TintButton,
            width = percent(100),
            height = auto(),
            justify = JustifyContent::FlexStart,
            padding = UiRect::axes(px(4), px(3)),
            radius = px(4),
            icon = val!(
                Icon,
                image = icons::CHEVRON,
                color = theme.text_muted,
                rotation = CHEVRON_OPEN
            ),
            label = val!(
                Label,
                text = name,
                color = Some(theme.text_primary),
                bold = true
            )
        ))
        .observe(move |_: On<Activate>, mut commands: Commands| {
            let path = click_path.clone();
            commands.queue(move |world: &mut World| {
                toggle_folded(world, target, path);
            });
        })
        .bind(
            |button| button.icon().rotation(),
            fold_changed(target, glyph_path.clone()),
            move |world, _| {
                if is_folded(world, target, &glyph_path) {
                    CHEVRON_FOLDED
                } else {
                    CHEVRON_OPEN
                }
            },
        );

        let body_path = path.clone();
        ui.elem(elem!(
            Frame,
            width = percent(100),
            direction = FlexDirection::Row,
            align = AlignItems::Stretch
        ))
        .bind(
            |frame| frame.display(),
            fold_changed(target, body_path.clone()),
            move |world, _| {
                if is_folded(world, target, &body_path) {
                    Display::None
                } else {
                    Display::Flex
                }
            },
        )
        .with(move |ui| {
            // A guide rail beside the group's own fields, the way a
            // tree view marks how deep a nested row sits - stretched
            // to the block's height rather than sized by hand.
            ui.elem(elem!(
                Frame,
                width = px(1),
                background = theme.palette.base[2]
            ));
            ui.elem(elem!(
                Frame,
                direction = FlexDirection::Column,
                flex_grow = 1.0f32,
                row_gap = px(4),
                padding = UiRect::new(
                    px(9),
                    Val::ZERO,
                    Val::ZERO,
                    Val::ZERO
                )
            ))
            .with(move |ui| {
                build_entries(ui, target, children);
            });
        });
    });
}
