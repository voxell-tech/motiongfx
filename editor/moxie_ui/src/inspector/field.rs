use std::any::TypeId;

use bevy::ecs::change_detection::{ComponentTicks, Tick};
use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::{GetPath, PartialReflect};

use super::Source;

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
/// One [`Source`] a widget can be handed. It deliberately carries no
/// value - a widget re-reads through the path whenever the component
/// changes, so nothing goes stale behind a snapshot.
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
}

/// The leaf, read and written through reflection.
///
/// Change detection rides the component's tick rather than the value,
/// so polling costs a lookup instead of a reflect read every frame.
impl Source for Field {
    fn get(&self, world: &World) -> Option<Box<dyn PartialReflect>> {
        self.read(world, |value| {
            Some(self.resolve(value)?.to_dynamic())
        })
        .flatten()
    }

    fn set(&self, world: &mut World, value: &dyn PartialReflect) {
        self.write(world, |component| {
            let path = &*self.path;
            let leaf = if path.is_empty() {
                Ok(component.as_partial_reflect_mut())
            } else {
                component.reflect_path_mut(path)
            };
            match leaf {
                Ok(leaf) => {
                    if let Err(err) = leaf.try_apply(value) {
                        warn!("inspector could not write {path}: {err:?}");
                    }
                }
                Err(err) => {
                    warn!("inspector lost the path {path}: {err:?}")
                }
            }
        });
    }

    fn changed(
        &self,
    ) -> Box<dyn FnMut(&World) -> bool + Send + Sync> {
        let field = self.clone();
        Box::new(crate::reactive::tick_changed(move |world| {
            field.changed_tick(world)
        }))
    }

    fn boxed(&self) -> Box<dyn Source> {
        Box::new(self.clone())
    }
}
