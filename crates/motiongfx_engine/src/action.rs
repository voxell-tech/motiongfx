use core::marker::PhantomData;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::field::UntypedField;
use crate::pipeline::PipelineKey;
use crate::track::{ActionKey, TrackFragment};
use crate::ThreadSafe;

#[allow(clippy::type_complexity)]
#[derive(Default)]
pub struct ActionWorld {
    world: World,
    pipeline_counts: HashMap<PipelineKey, u32>,
}

impl ActionWorld {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ActionWorld {
    pub fn add<T>(
        &mut self,
        action: impl Action<T>,
        target: impl Into<ActionTarget>,
        field: impl Into<UntypedField>,
    ) -> ActionBuilder<'_, T>
    where
        T: ThreadSafe,
    {
        let field = field.into();
        let key = PipelineKey::from_field(field);

        match self.pipeline_counts.get_mut(&key) {
            Some(count) => *count += 1,
            None => {
                self.pipeline_counts.insert(key, 1);
            }
        }

        let key = ActionKey {
            target: target.into(),
            field,
        };
        let world =
            self.world.spawn((key, ActionStorage::new(action)));

        ActionBuilder {
            world,
            key,
            _phantom: PhantomData,
        }
    }

    pub fn remove(&mut self, id: ActionId) -> bool {
        let entity = id.entity();

        // Early check if the action exists.
        if self.world.get_entity(entity).is_err() {
            return false;
        }

        let field = self
            .world
            .get::<ActionKey>(entity)
            .expect("All actions should have an `ActionKey`!")
            .field;

        let key = PipelineKey::from_field(field);

        let count =
            self.pipeline_counts.get_mut(&key).unwrap_or_else(|| {
                panic!("Field counts not registered for {field:?}!")
            });

        *count -= 1;
        if *count == 0 {
            self.pipeline_counts.remove(&key);
        }

        self.world.despawn(id.entity())
    }

    pub fn get_action<T>(
        &self,
        id: ActionId,
    ) -> Option<&impl Action<T>>
    where
        T: ThreadSafe,
    {
        self.world
            .get::<ActionStorage<T>>(id.entity())
            .map(|a| &a.action)
    }

    pub fn get_segment<T>(&self, id: ActionId) -> Option<&Segment<T>>
    where
        T: ThreadSafe,
    {
        self.world.get::<Segment<T>>(id.entity())
    }
}

impl ActionWorld {
    pub(crate) fn world(&self) -> &World {
        &self.world
    }

    pub(crate) fn with_action(
        &mut self,
        id: ActionId,
    ) -> Option<EntityWorldMut<'_>> {
        self.world.get_entity_mut(id.entity()).ok()
    }
}

pub struct ActionCommand<'w> {
    world: EntityWorldMut<'w>,
}

impl ActionCommand<'_> {
    // pub fn mark(
    //     &mut self,
    //     sample_mode: SampleMode,
    // ) -> &mut Self {
    //     self.world.insert(sample_mode);
    //     self
    // }

    // pub fn clear_mark(&mut self) -> &mut Self {
    //     self.world.remove::<SampleMode>();
    //     self
    // }
}

pub struct ActionBuilder<'w, T> {
    world: EntityWorldMut<'w>,
    key: ActionKey,
    _phantom: PhantomData<T>,
}

impl<'w, T> ActionBuilder<'w, T> {
    /// Get the [`ActionId`] of the containing action.
    pub fn id(&self) -> ActionId {
        ActionId::new(self.world.id())
    }
}

impl<T> ActionBuilder<'_, T>
where
    T: 'static,
{
    pub fn with_ease(mut self, ease: EaseFn) -> Self {
        self.world.insert(EaseStorage(ease));
        self
    }

    pub fn with_interp(mut self, interp: InterpFn<T>) -> Self {
        self.world.insert(InterpStorage(interp));
        self
    }

    pub fn play(self, duration: f32) -> TrackFragment {
        TrackFragment::single(
            self.key,
            ActionClip::new(self.id(), duration),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionId(Entity);

impl ActionId {
    pub const PLACEHOLDER: Self = ActionId(Entity::PLACEHOLDER);

    #[inline(always)]
    pub(crate) fn new(entity: Entity) -> Self {
        Self(entity)
    }

    #[inline(always)]
    pub(crate) fn entity(&self) -> Entity {
        self.0
    }
}

/// An action trait which consists of a function for getting
/// the target value based on an intial value.
pub trait Action<T>: ThreadSafe + Fn(&T) -> T {}

impl<T, U> Action<T> for U where U: ThreadSafe + Fn(&T) -> T {}

/// A storage component for an [`Action`].
#[derive(Component)]
#[component(immutable)]
pub struct ActionStorage<T> {
    pub action: Box<dyn Action<T>>,
}

impl<T> ActionStorage<T> {
    pub fn new(action: impl Action<T>) -> Self {
        Self {
            action: Box::new(action),
        }
    }
}

/// Function for interpolating a type based on a [`f32`] time.
pub type InterpFn<T> = fn(start: &T, end: &T, t: f32) -> T;

/// A storage component for a custom [`InterpFn`].
///
/// This can be optionally inserted alongside [`ActionStorage`]
/// to customize the action.
#[derive(Component, Deref, Debug, Clone, Copy)]
#[component(immutable)]
pub struct InterpStorage<T>(pub InterpFn<T>);

/// Easing function on a [`f32`] time.
pub type EaseFn = fn(t: f32) -> f32;

/// A storage component for a custom [`EaseFn`].
///
/// This can be optionally inserted alongside [`ActionStorage`]
/// to customize the action.
#[derive(Component, Deref, Debug, Clone, Copy)]
#[component(immutable)]
pub struct EaseStorage(pub EaseFn);

#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct ActionTarget(pub Entity);

impl From<Entity> for ActionTarget {
    fn from(entity: Entity) -> Self {
        Self(entity)
    }
}

#[derive(Component)]
#[component(immutable)]
pub struct Segment<T> {
    /// The starting value.
    pub start: T,
    /// The ending value.
    pub end: T,
}

impl<T> Segment<T> {
    pub fn new(start: T, end: T) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActionClip {
    pub id: ActionId,
    pub start: f32,
    pub duration: f32,
}

impl ActionClip {
    pub const fn new(id: ActionId, duration: f32) -> Self {
        Self {
            id,
            start: 0.0,
            duration,
        }
    }

    #[inline]
    pub fn end(&self) -> f32 {
        self.start + self.duration
    }
}
