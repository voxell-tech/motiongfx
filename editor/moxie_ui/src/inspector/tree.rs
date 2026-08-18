//! Walks a component's reflected value into a collapsible hierarchy
//! and renders it.
//!
//! A struct with no [`ReflectInspect`] of its own is not a leaf: its
//! fields become a group, shown under a header that folds them away.
//! Only a registered type stops the walk and becomes an editable row.

use std::any::TypeId;

use bevy::ecs::change_detection::Tick;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::reflect::{PartialReflect, ReflectRef, TypeRegistry};
use bevy_fynix::ElementMutExt;
use fynix_mock::composer::Composer;
use fynix_mock::ui::{ElementHandle, ElementMut};
use fynix_mock::{elem, val};

use super::{Field, ReflectInspect, enums};
use crate::elements::{ButtonElem, Frame, Icon, Label, TintButton};
use crate::fold::{CHEVRON_OPEN, Foldable, Toggle};
use crate::icons;
use crate::reactive::{BevyHost, BevyUi};
use crate::theme::EditorTheme;

/// One row the walk found.
#[derive(Clone, PartialEq)]
enum Entry {
    /// A registered widget draws it.
    Leaf { path: String, type_id: TypeId },
    /// A struct, its fields under a folding header.
    Group {
        path: String,
        name: String,
        children: Vec<Entry>,
    },
    /// An enum: a variant picker, and the active variant's fields
    /// when it has any.
    Variant {
        path: String,
        name: String,
        variants: Vec<String>,
        /// Only unit variants can be picked; see [`enums`].
        pick: bool,
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

    if let Some(variants) = enums::variants(value) {
        let pick = enums::constructible(value, registry);
        out.push(Entry::Variant {
            path: path.to_string(),
            name: name.to_string(),
            variants,
            pick,
            children: collect_entries(registry, value, path),
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
        // Whatever the active variant carries, named the way a path
        // reaches it - by field for a struct variant, by index for a
        // tuple one.
        ReflectRef::Enum(value) => {
            for i in 0..value.field_len() {
                let Some(field) = value.field_at(i) else {
                    continue;
                };
                let name = value
                    .name_at(i)
                    .map(str::to_string)
                    .unwrap_or_else(|| i.to_string());
                push_entry(
                    registry,
                    field,
                    &join(prefix, &name),
                    &name,
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

/// The entries under `field`, in walk order.
///
/// Unlike a nested field, the root itself is never wrapped in a
/// group: if it is a leaf type it is the one row shown, and otherwise
/// its fields are listed directly rather than folded under a header
/// for a group that was never named.
fn entries(world: &World, field: &Field) -> Vec<Entry> {
    let mut out = Vec::new();
    field.read(world, |value| {
        // Taken inside the read, where `Field` has already released
        // its own guard.
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
        } else if let Some(variants) = enums::variants(value) {
            out.push(Entry::Variant {
                path: String::new(),
                name: String::new(),
                variants,
                pick: enums::constructible(value, &registry),
                children: collect_entries(&registry, value, ""),
            });
        } else {
            out = collect_entries(&registry, value, "");
        }
    });
    out
}

/// Fires when the *shape* under `field` changes, meaning its set of
/// entries, and not merely their values.
///
/// Values ride on bindings rather than rebuilds, which is what lets a
/// number input keep focus while the value changes underneath it: a
/// rebuild would despawn the widget mid-edit. The tick is checked
/// first so the reflection walk only runs when something actually
/// touched the component.
fn shape_changed(field: Field) -> impl FnMut(&World, Entity) -> bool {
    let mut seen_tick: Option<Tick> = None;
    let mut seen_shape: Option<Vec<Entry>> = None;
    move |world, _| {
        let tick = field.changed_tick(world);
        if seen_shape.is_some() && tick == seen_tick {
            return false;
        }
        seen_tick = tick;

        let current = entries(world, &field);
        let fired = seen_shape.as_ref() != Some(&current);
        seen_shape = Some(current);
        fired
    }
}

/// Editable rows for everything reflectable under `root`, which is a
/// whole component at the empty path.
pub struct InspectorFields {
    pub root: Field,
}

impl Composer<BevyHost> for InspectorFields {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        let walked = self.root.clone();

        ui.elem(elem!(
            Frame,
            width = percent(100),
            direction = FlexDirection::Column,
            row_gap = px(4)
        ))
        .insert(TabGroup::new(0))
        .watch(shape_changed(self.root), move |ui| {
            build_entries(ui, &walked, entries(ui.world, &walked));
        })
        .handle()
    }
}

fn build_entries(ui: &mut BevyUi, root: &Field, entries: Vec<Entry>) {
    for entry in entries {
        match entry {
            Entry::Leaf { path, type_id } => {
                build_leaf(ui, root, path, type_id)
            }
            Entry::Group { name, children, .. } => {
                build_group(ui, root, name, children)
            }
            Entry::Variant {
                path,
                name,
                variants,
                pick,
                children,
            } => build_variant(
                ui, root, path, name, variants, pick, children,
            ),
        }
    }
}

fn build_leaf(
    ui: &mut BevyUi,
    root: &Field,
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
    let field = root.child(&path);
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

/// A variant picker, and - for an enum carrying data - the active
/// variant's own fields folded underneath it.
fn build_variant(
    ui: &mut BevyUi,
    root: &Field,
    path: String,
    name: String,
    variants: Vec<String>,
    pick: bool,
    children: Vec<Entry>,
) {
    let field = root.child(&path);
    let muted = ui.world.resource::<EditorTheme>().text_muted;

    if children.is_empty() {
        let label = leaf_name(&path).to_string();
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
            ui.compose(enums::VariantPicker {
                source: &field,
                variants,
                pick,
            });
        });
        return;
    }

    // The root has no name to head a group with - see `entries`.
    if path.is_empty() {
        ui.compose(enums::VariantPicker {
            source: &field,
            variants,
            pick,
        });
        build_entries(ui, root, children);
        return;
    }

    let inner = root.clone();
    ui.compose(Section {
        name,
        body: move |ui: &mut BevyUi| {
            ui.compose(enums::VariantPicker {
                source: &field,
                variants,
                pick,
            });
            build_entries(ui, &inner, children);
        },
    });
}

fn build_group(
    ui: &mut BevyUi,
    root: &Field,
    name: String,
    children: Vec<Entry>,
) {
    let inner = root.clone();

    ui.compose(Section {
        name,
        body: move |ui: &mut BevyUi| {
            build_entries(ui, &inner, children);
        },
    });
}

/// A collapsible section: a header that folds it, and a body indented
/// under a guide rail.
pub struct Section<F> {
    pub name: String,
    pub body: F,
}

impl<F: FnOnce(&mut BevyUi)> Composer<BevyHost> for Section<F> {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        let Self { name, body } = self;
        let theme = ui.world.resource::<EditorTheme>().clone();

        ui.compose(Foldable {
            header: elem!(
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
            ),
            // Nothing else to mean: the whole header folds it.
            toggle: Toggle::Header,
            enabled: true,
            on_header: |_: ElementMut<
                '_,
                '_,
                BevyHost,
                ButtonElem,
            >| {},
            body,
        })
        .handle()
    }
}
