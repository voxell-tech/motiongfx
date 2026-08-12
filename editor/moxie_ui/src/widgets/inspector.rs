//! Reflection-driven inspector.
//!
//! [`inspector_fields`] walks any reflected value in the world and
//! renders its leaves as editable rows. Which widget a leaf gets is a
//! type-registry lookup, not a match on concrete types, so a new
//! editable type is one [`Inspect`] impl away - see [`widget`].
//!
//! An [`InspectorTarget`] says where the value lives. Bevy stores a
//! resource as a component on an entity of its own, so a resource and
//! a component are the same lookup once the entity is resolved, and
//! the inspector never needs to know which it was handed.

pub mod widget;

use std::any::TypeId;
use std::sync::Arc;

use bevy::ecs::change_detection::{ComponentTicks, Tick};
use bevy::ecs::reflect::ReflectComponent;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::reflect::{GetPath, ReflectRef, TypeRegistry};

use bevy_fynix::ElementMutExt;
use fynix_mock::elem;

use crate::fynix::{Frame, Label};
use crate::reactive::BevyUi;
pub use widget::{Inspect, InspectAppExt, ReflectInspect};

/// Where an inspector reads and writes the value it edits.
///
/// Holds a [`TypeId`] rather than a `ComponentId` so a target can be
/// named before the world has registered the type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InspectorTarget {
    /// A resource, which bevy keeps on an entity of its own.
    Resource(TypeId),
    /// One component of one entity.
    Component { entity: Entity, type_id: TypeId },
}

impl InspectorTarget {
    pub fn resource<T: Resource + Reflect>() -> Self {
        Self::Resource(TypeId::of::<T>())
    }

    pub fn component<T: Component + Reflect>(entity: Entity) -> Self {
        Self::Component {
            entity,
            type_id: TypeId::of::<T>(),
        }
    }

    pub fn type_id(&self) -> TypeId {
        match *self {
            Self::Resource(type_id) => type_id,
            Self::Component { type_id, .. } => type_id,
        }
    }

    /// The entity actually holding the value, which for a resource is
    /// the one bevy parks it on.
    fn entity(&self, world: &World) -> Option<Entity> {
        match *self {
            Self::Component { entity, .. } => Some(entity),
            Self::Resource(type_id) => {
                let id = world.components().get_id(type_id)?;
                world.resource_entities().get(id)
            }
        }
    }

    /// The accessor for this target's type, cloned out so the registry
    /// guard is never held while the caller runs. Nesting two read
    /// guards on one thread can deadlock the moment a writer queues
    /// between them, and callers here reach for the registry again.
    fn accessor(&self, world: &World) -> Option<ReflectComponent> {
        let registry = world.resource::<AppTypeRegistry>().read();
        registry
            .get_type_data::<ReflectComponent>(self.type_id())
            .cloned()
    }

    /// Runs `read` against the value, or returns `None` when the
    /// target is gone or its type was never registered with
    /// `#[reflect(Component)]` / `#[reflect(Resource)]`.
    pub fn read<R>(
        &self,
        world: &World,
        read: impl FnOnce(&dyn Reflect) -> R,
    ) -> Option<R> {
        let entity = self.entity(world)?;
        let component = self.accessor(world)?;
        let value =
            component.reflect(world.get_entity(entity).ok()?)?;
        Some(read(value))
    }

    /// Runs `write` against the value.
    pub fn write<R>(
        &self,
        world: &mut World,
        write: impl FnOnce(&mut dyn Reflect) -> R,
    ) -> Option<R> {
        let entity = self.entity(world)?;
        let component = self.accessor(world)?;
        let mut entity = world.get_entity_mut(entity).ok()?;
        let mut value = component.reflect_mut(&mut entity)?;
        Some(write(&mut *value))
    }

    /// The tick the value last changed on, which is what the bindings
    /// poll instead of re-reading through reflection every frame.
    fn changed_tick(&self, world: &World) -> Option<Tick> {
        let entity = self.entity(world)?;
        let id = world.components().get_id(self.type_id())?;
        let ComponentTicks { changed, .. } = world
            .get_entity(entity)
            .ok()?
            .get_change_ticks_by_id(id)?;
        Some(changed)
    }
}

/// Fires when `target`'s value changed since the last poll, and on the
/// first poll so a binding starts out in sync with the world.
pub fn target_changed(
    target: InspectorTarget,
) -> impl FnMut(&World, Entity) -> bool {
    let mut seen: Option<Tick> = None;
    let mut polled = false;
    move |world, _| {
        let current = target.changed_tick(world);
        let fired = !polled || seen != current;
        seen = current;
        polled = true;
        fired
    }
}

/// One editable leaf: a target, and the reflect path reaching the leaf
/// inside it.
///
/// This is what an [`Inspect`] widget binds to. It deliberately
/// carries no value - a widget re-reads through the path whenever the
/// target changes, so nothing goes stale behind a snapshot.
#[derive(Clone, Debug)]
pub struct Field {
    target: InspectorTarget,
    path: Arc<str>,
}

impl Field {
    pub fn new(
        target: InspectorTarget,
        path: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            target,
            path: path.into(),
        }
    }

    pub fn target(&self) -> InspectorTarget {
        self.target
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    /// The leaf's current value, cloned out of the target.
    pub fn get<T: FromReflect>(&self, world: &World) -> Option<T> {
        self.target
            .read(world, |value| {
                T::from_reflect(self.resolve(value)?)
            })
            .flatten()
    }

    /// Overwrites the leaf, leaving the target untouched if the path
    /// no longer resolves or the types disagree.
    pub fn set<T: PartialReflect>(
        &self,
        world: &mut World,
        value: T,
    ) {
        self.target.write(world, |target| {
            let path = &*self.path;
            let leaf = if path.is_empty() {
                Ok(target.as_partial_reflect_mut())
            } else {
                target.reflect_path_mut(path)
            };
            match leaf {
                Ok(leaf) => {
                    if let Err(err) = leaf.try_apply(value.as_partial_reflect())
                    {
                        warn!("inspector could not write {path}: {err:?}");
                    }
                }
                Err(err) => warn!("inspector lost the path {path}: {err:?}"),
            }
        });
    }

    /// The leaf inside an already-read target. An empty path is the
    /// target itself, which `reflect_path` does not accept.
    fn resolve<'a>(
        &self,
        value: &'a dyn Reflect,
    ) -> Option<&'a dyn PartialReflect> {
        if self.path.is_empty() {
            Some(value.as_partial_reflect())
        } else {
            value.reflect_path(&*self.path).ok()
        }
    }
}

/// A leaf found by the walk, and the type whose widget draws it.
#[derive(Clone, PartialEq)]
struct Leaf {
    path: String,
    type_id: TypeId,
}

/// Flattens `value` into the leaves that have a widget registered.
///
/// A type with a [`ReflectInspect`] is a leaf even when it is a
/// struct: registering a widget is how a type says it presents itself,
/// so the walk stops rather than exposing its innards.
fn collect_leaves(
    registry: &TypeRegistry,
    value: &dyn PartialReflect,
    prefix: &str,
    out: &mut Vec<Leaf>,
) {
    if let Some(type_id) =
        value.get_represented_type_info().map(|i| i.type_id())
        && registry.get_type_data::<ReflectInspect>(type_id).is_some()
    {
        out.push(Leaf {
            path: prefix.to_string(),
            type_id,
        });
        return;
    }

    match value.reflect_ref() {
        ReflectRef::Struct(value) => {
            for i in 0..value.field_len() {
                let (Some(name), Some(field)) =
                    (value.name_at(i), value.field_at(i))
                else {
                    continue;
                };
                collect_leaves(
                    registry,
                    field,
                    &join(prefix, name),
                    out,
                );
            }
        }
        ReflectRef::TupleStruct(value) => {
            for i in 0..value.field_len() {
                let Some(field) = value.field(i) else {
                    continue;
                };
                collect_leaves(
                    registry,
                    field,
                    &join(prefix, &i.to_string()),
                    out,
                );
            }
        }
        _ => {}
    }
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}.{name}")
    }
}

/// Every leaf of `target`, in walk order.
fn leaves(world: &World, target: InspectorTarget) -> Vec<Leaf> {
    let mut out = Vec::new();
    target.read(world, |value| {
        // Taken inside the read, where `InspectorTarget` has already
        // released its own guard.
        let registry = world.resource::<AppTypeRegistry>().read();
        collect_leaves(
            &registry,
            value.as_partial_reflect(),
            "",
            &mut out,
        );
    });
    out
}

/// Fires when the *shape* of `target` changes, meaning its set of
/// leaves, and not merely their values.
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
    let mut seen_shape: Option<Vec<Leaf>> = None;
    move |world, _| {
        let tick = target.changed_tick(world);
        if seen_shape.is_some() && tick == seen_tick {
            return false;
        }
        seen_tick = tick;

        let current = leaves(world, target);
        let fired = seen_shape.as_ref() != Some(&current);
        seen_shape = Some(current);
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
        build_fields(ui, target);
    });
}

fn build_fields(ui: &mut BevyUi, target: InspectorTarget) {
    for leaf in leaves(ui.world, target) {
        let drawer = {
            let registry =
                ui.world.resource::<AppTypeRegistry>().read();
            registry
                .get_type_data::<ReflectInspect>(leaf.type_id)
                .cloned()
        };
        let Some(drawer) = drawer else { continue };

        let field = Field::new(target, leaf.path.clone());
        let label = leaf.path;
        ui.elem(elem!(
            Frame,
            width = percent(100),
            direction = FlexDirection::Row,
            justify = JustifyContent::SpaceBetween,
            align = AlignItems::Center,
            column_gap = px(8),
            padding = UiRect::vertical(px(2))
        ))
        .with(move |ui| {
            ui.elem(elem!(Label, text = label));
            drawer.build(&field, ui);
        });
    }
}
