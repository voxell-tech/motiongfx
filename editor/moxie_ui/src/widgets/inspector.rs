//! Reflection-driven inspector.
//!
//! [`inspector_fields`] walks any reflected value in the world and
//! renders it as a collapsible hierarchy of editable rows. Which
//! widget a leaf gets is a type-registry lookup, not a match on
//! concrete types, so a new editable type is one [`Inspect`] impl
//! away.
//!
//! An [`InspectorTarget`] says where the value lives. Bevy stores a
//! resource as a component on an entity of its own, so a resource and
//! a component are the same lookup once the entity is resolved, and
//! the inspector never needs to know which it was handed.

mod primitive;
mod tree;
mod vector;

use std::any::TypeId;
use std::sync::Arc;

use bevy::ecs::change_detection::{ComponentTicks, Tick};
use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::{FromType, GetPath, GetTypeRegistration};

use crate::reactive::BevyUi;
pub use tree::inspector_fields;

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

/// Builds the editing widget for one reflected value.
///
/// The value itself is not passed in. A widget is built once and then
/// binds to `field`, re-reading through it whenever the target
/// changes, which is what keeps a focused input alive across edits.
pub trait Inspect: Reflect + TypePath + GetTypeRegistration {
    fn build(field: &Field, ui: &mut BevyUi);
}

/// Type data pointing at a type's [`Inspect::build`].
///
/// The function needs no downcast, unlike a drawer that takes the
/// value: `build` is resolved from the type it was registered for, and
/// the widget reads its own value back through [`Field`].
#[derive(Clone)]
pub struct ReflectInspect {
    build: fn(&Field, &mut BevyUi),
}

impl ReflectInspect {
    pub fn build(&self, field: &Field, ui: &mut BevyUi) {
        (self.build)(field, ui)
    }
}

impl<T: Inspect> FromType<T> for ReflectInspect {
    fn from_type() -> Self {
        Self { build: T::build }
    }
}

/// Registering inspector widgets on the app.
pub trait InspectAppExt {
    /// Makes `T` editable wherever the inspector meets it.
    fn register_inspect<T: Inspect>(&mut self) -> &mut Self;

    /// Registers the primitives the inspector can edit out of the box.
    fn register_default_inspects(&mut self) -> &mut Self;
}

impl InspectAppExt for App {
    fn register_inspect<T: Inspect>(&mut self) -> &mut Self {
        self.register_type::<T>()
            .register_type_data::<T, ReflectInspect>()
    }

    fn register_default_inspects(&mut self) -> &mut Self {
        self.register_inspect::<bool>()
            .register_inspect::<f32>()
            .register_inspect::<f64>()
            .register_inspect::<i32>()
            .register_inspect::<i64>()
            .register_inspect::<u32>()
            .register_inspect::<u64>()
            .register_inspect::<Vec2>()
            .register_inspect::<Vec3>()
            .register_inspect::<Vec4>()
            .register_inspect::<IVec2>()
            .register_inspect::<IVec3>()
            .register_inspect::<IVec4>()
            .register_inspect::<UVec2>()
            .register_inspect::<UVec3>()
            .register_inspect::<UVec4>()
    }
}
