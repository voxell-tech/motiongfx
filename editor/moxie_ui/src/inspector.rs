//! Reflection-driven inspector.
//!
//! [`inspector_fields`] walks any reflected value in the world and
//! renders it as a collapsible hierarchy of editable rows. Which
//! widget a leaf gets is a type-registry lookup, not a match on
//! concrete types, so a new editable type is one [`Inspect`] impl
//! away.
//!
//! A [`Field`] says where the value lives: one component of one
//! entity, and the reflect path to a leaf inside it. Bevy keeps a
//! resource on an entity of its own, so a resource is a component
//! too, and nothing here needs to know which it was handed.

mod primitive;
mod tree;
mod vector;

use std::any::TypeId;

use bevy::ecs::change_detection::{ComponentTicks, Tick};
use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::{FromType, GetPath, GetTypeRegistration};

use crate::reactive::BevyUi;
pub use tree::{inspector_fields, section};

/// Where an inspector reads and writes: one component of one entity,
/// and the reflect path reaching a leaf inside it. The empty path is
/// the component itself.
///
/// A resource is a component too - bevy parks each one on an entity
/// of its own, and `#[reflect(Resource)]` registers a
/// [`ReflectComponent`] alongside - so which it was handed never
/// comes up. *Which* entity that is gets settled once, by whoever
/// built the field, rather than looked up on every read: it only
/// moves when the resource is removed and re-inserted, and a value
/// that has been replaced wants the subtree rebuilt, not its bindings
/// quietly re-pointed at a new instance.
///
/// This is what an [`Inspect`] widget binds to. It deliberately
/// carries no value - a widget re-reads through the path whenever the
/// component changes, so nothing goes stale behind a snapshot.
///
/// Holds a [`TypeId`] rather than a `ComponentId` so a field can be
/// named before the world has registered the type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Field {
    entity: Entity,
    component: TypeId,
    path: Box<str>,
}

impl Field {
    /// The whole of one component, which is the empty path.
    pub fn new(entity: Entity, component: TypeId) -> Self {
        Self {
            entity,
            component,
            path: "".into(),
        }
    }

    /// The whole of one component, named by type.
    pub fn of<T: Component + Reflect>(entity: Entity) -> Self {
        Self::new(entity, TypeId::of::<T>())
    }

    /// The leaf one step further in, which is how the walk descends.
    pub fn child(&self, name: &str) -> Self {
        let path = if self.path.is_empty() {
            name.to_string()
        } else {
            format!("{}.{name}", self.path)
        };

        Self {
            entity: self.entity,
            component: self.component,
            path: path.into_boxed_str(),
        }
    }

    /// The same component, back at its root.
    pub fn root(&self) -> Self {
        Self::new(self.entity, self.component)
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn component(&self) -> TypeId {
        self.component
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    /// The accessor for this field's component, cloned out so the
    /// registry guard is never held while the caller runs. Nesting
    /// two read guards on one thread can deadlock the moment a writer
    /// queues between them, and callers here reach for the registry
    /// again.
    fn accessor(&self, world: &World) -> Option<ReflectComponent> {
        let registry = world.resource::<AppTypeRegistry>().read();
        registry
            .get_type_data::<ReflectComponent>(self.component)
            .cloned()
    }

    /// Runs `read` against the whole component, or returns `None`
    /// when the entity is gone, does not carry it, or its type was
    /// never registered with `#[reflect(Component)]` /
    /// `#[reflect(Resource)]`.
    pub fn read<R>(
        &self,
        world: &World,
        read: impl FnOnce(&dyn Reflect) -> R,
    ) -> Option<R> {
        let component = self.accessor(world)?;
        let value =
            component.reflect(world.get_entity(self.entity).ok()?)?;
        Some(read(value))
    }

    /// Runs `write` against the whole component.
    pub fn write<R>(
        &self,
        world: &mut World,
        write: impl FnOnce(&mut dyn Reflect) -> R,
    ) -> Option<R> {
        let component = self.accessor(world)?;
        let mut entity = world.get_entity_mut(self.entity).ok()?;
        let mut value = component.reflect_mut(&mut entity)?;
        Some(write(&mut *value))
    }

    /// Whether the component is there at all.
    pub fn exists(&self, world: &World) -> bool {
        let Ok(entity) = world.get_entity(self.entity) else {
            return false;
        };
        let Some(component) = self.accessor(world) else {
            return false;
        };

        component.contains(entity)
    }

    /// The leaf's current value, cloned out of the component.
    pub fn get<T: FromReflect>(&self, world: &World) -> Option<T> {
        self.read(world, |value| {
            T::from_reflect(self.resolve(value)?)
        })
        .flatten()
    }

    /// Overwrites the leaf, leaving the component untouched if the
    /// path no longer resolves or the types disagree.
    pub fn set<T: PartialReflect>(
        &self,
        world: &mut World,
        value: T,
    ) {
        self.write(world, |component| {
            let path = &*self.path;
            let leaf = if path.is_empty() {
                Ok(component.as_partial_reflect_mut())
            } else {
                component.reflect_path_mut(path)
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

    /// The leaf inside an already-read component. An empty path is
    /// the component itself, which `reflect_path` does not accept.
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

    /// The tick the component last changed on, which is what the
    /// bindings poll instead of re-reading through reflection every
    /// frame.
    pub(crate) fn changed_tick(&self, world: &World) -> Option<Tick> {
        let id = world.components().get_id(self.component)?;
        let ComponentTicks { changed, .. } = world
            .get_entity(self.entity)
            .ok()?
            .get_change_ticks_by_id(id)?;
        Some(changed)
    }

    /// Fires when the component this field sits in changed since the
    /// last poll, and on the first poll so a binding starts out in
    /// sync with the world.
    pub fn changed(self) -> impl FnMut(&World, Entity) -> bool {
        let field = self;
        let mut seen: Option<Tick> = None;
        let mut polled = false;
        move |world, _| {
            let current = field.changed_tick(world);
            let fired = !polled || seen != current;
            seen = current;
            polled = true;
            fired
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
