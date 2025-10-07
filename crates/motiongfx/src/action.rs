use core::marker::PhantomData;

use alloc::boxed::Box;
use alloc::vec::Vec;
use bevy_ecs::prelude::*;

use crate::field::UntypedField;
use crate::subject::SubjectId;
use crate::track::TrackFragment;
use crate::ThreadSafe;

#[allow(clippy::type_complexity)]
#[derive(Default)]
pub struct ActionWorld<I: SubjectId> {
    world: World,
    _marker: PhantomData<I>,
}

impl<I: SubjectId> ActionWorld<I> {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            _marker: PhantomData,
        }
    }

    pub fn add<T>(
        &mut self,
        target: I,
        field: impl Into<UntypedField>,
        action: impl Action<T>,
    ) -> ActionBuilder<'_, I, T>
    where
        T: ThreadSafe,
    {
        let field = field.into();

        let key = ActionKey { target, field };
        let world =
            self.world.spawn((key, ActionStorage::new(action)));

        ActionBuilder {
            world,
            key,
            _phantom: PhantomData,
        }
    }

    pub fn remove(&mut self, id: ActionId) -> Option<ActionKey<I>> {
        let entity = id.entity();

        let key = *self
            .world
            .get_entity(entity)
            .ok()?
            .get::<ActionKey<I>>()
            .expect("All actions should have an `ActionKey`!");

        self.world.despawn(id.entity());

        Some(key)
    }

    pub fn get<T>(&self, id: ActionId) -> Option<&impl Action<T>>
    where
        T: ThreadSafe,
    {
        self.world
            .get::<ActionStorage<T>>(id.entity())
            .map(|a| &a.action)
    }
}

impl<I: SubjectId> ActionWorld<I> {
    /// Returns a immutable reference to the underlying world.
    pub(crate) fn world(&self) -> &World {
        &self.world
    }

    /// Create an [`ActionCommand`] from an [`ActionId`].
    ///
    /// # Panics
    ///
    /// Panics if the action does not exists in the world.
    ///
    /// In general, this should not be an issue as this is only used
    /// internally within the crate.
    pub(crate) fn edit_action(
        &'_ mut self,
        id: ActionId,
    ) -> ActionCommand<'_> {
        ActionCommand {
            world: self.world.entity_mut(id.entity()),
        }
    }

    /// Remove [`SampleMode`] component from all marked actions.
    pub(crate) fn clear_all_marks(&mut self) {
        let Some(mut q) = self
            .world
            .try_query_filtered::<Entity, With<SampleMode>>()
        else {
            return;
        };

        let entities = q.iter(&self.world).collect::<Vec<_>>();
        for entity in entities {
            self.world.entity_mut(entity).remove::<SampleMode>();
        }
    }
}

pub(crate) struct ActionCommand<'w> {
    world: EntityWorldMut<'w>,
}

impl ActionCommand<'_> {
    pub(crate) fn mark(
        &mut self,
        sample_mode: SampleMode,
    ) -> &mut Self {
        self.world.insert(sample_mode);
        self
    }

    pub(crate) fn clear_mark(&mut self) -> &mut Self {
        self.world.remove::<SampleMode>();
        self
    }

    /// Add or replace the segment of the action.
    pub(crate) fn set_segment<T>(
        &mut self,
        segment: Segment<T>,
    ) -> &mut Self
    where
        T: ThreadSafe,
    {
        self.world.insert(segment);
        self
    }
}

pub struct ActionBuilder<'w, I, T>
where
    I: SubjectId,
{
    world: EntityWorldMut<'w>,
    key: ActionKey<I>,
    _phantom: PhantomData<T>,
}

/// A builder struct to insert an interpolation method for the action
/// before compiling into an [`InterpActionBuilder`].
impl<I, T> ActionBuilder<'_, I, T>
where
    I: SubjectId,
{
    /// Get the [`ActionId`] of the containing action.
    pub fn id(&self) -> ActionId {
        ActionId::new(self.world.id())
    }
}

impl<'w, I, T> ActionBuilder<'w, I, T>
where
    I: SubjectId,
    T: 'static,
{
    /// Set the interpolation method of the action.
    pub fn with_interp(
        mut self,
        interp: InterpFn<T>,
    ) -> InterpActionBuilder<'w, I, T> {
        self.world.insert(InterpStorage(interp));
        InterpActionBuilder { inner: self }
    }
}

/// An action builder that has interpolation added. This builder
/// exposes more customizations for the action and allows it to be
/// compiled into a [`TrackFragment`].
pub struct InterpActionBuilder<'w, I, T>
where
    I: SubjectId,
{
    inner: ActionBuilder<'w, I, T>,
}

impl<I, T> InterpActionBuilder<'_, I, T>
where
    I: SubjectId,
{
    /// Set the easing method of the action.
    pub fn with_ease(mut self, ease: EaseFn) -> Self {
        self.inner.world.insert(EaseStorage(ease));
        self
    }

    /// Get the [`ActionId`] of the containing action.
    pub fn id(&self) -> ActionId {
        self.inner.id()
    }

    /// Confirms the configuration of the action and creates a
    /// [`TrackFragment`].
    pub fn play(self, duration: f32) -> TrackFragment<I> {
        TrackFragment::single(
            self.inner.key,
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

/// Key that uniquely identifies a sequence of non-overlapping
/// actions.
#[derive(
    Component,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
#[component(immutable)]
pub struct ActionKey<I: SubjectId> {
    /// The target entity of the action.
    pub target: I,
    /// The source and target field related to the entity.
    pub field: UntypedField,
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
#[derive(Component, Debug, Clone, Copy)]
#[component(immutable)]
pub struct InterpStorage<T>(pub InterpFn<T>);

/// Easing function on a [`f32`] time.
pub type EaseFn = fn(t: f32) -> f32;

/// A storage component for a custom [`EaseFn`].
///
/// This can be optionally inserted alongside [`ActionStorage`]
/// to customize the action.
#[derive(Component, Debug, Clone, Copy)]
#[component(immutable)]
pub struct EaseStorage(pub EaseFn);

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

/// Determines how a [`Segment`] should be sampled.
#[derive(Component, Debug, Clone, Copy)]
#[component(storage = "SparseSet", immutable)]
pub enum SampleMode {
    Start,
    End,
    Interp(f32),
}
