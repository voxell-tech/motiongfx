use core::any::TypeId;
use core::marker::PhantomData;

use alloc::boxed::Box;
use alloc::vec::Vec;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::prelude::*;
use bevy_ecs::world::DeferredWorld;
use bevy_platform::collections::HashMap;

use crate::field::UntypedField;
use crate::subject::SubjectId;
use crate::track::TrackFragment;
use crate::ThreadSafe;

/// A type-erased unique Id in the [`IdRegistry`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct UId(u64);

/// A type-erased [`UId`] map and generator for each unique
/// [`SubjectId`]s. It also performs book keeping for all id instances
/// and remove them when there is none left.
#[derive(Resource)]
pub struct IdRegistry<I: SubjectId> {
    /// Maps `SubjectId`s to [`UId`]s .
    uid_map: HashMap<I, UId>,
    /// Maps [`UId`]s to `SubjectId`s.
    id_map: HashMap<UId, I>,
    /// The number of instances using the same [`UId`].
    instance_counts: HashMap<UId, u32>,
    /// The next [`UId`], incremented on every new [`UId`] created.
    next_uid: UId,
}

impl<I: SubjectId> IdRegistry<I> {
    pub fn new() -> Self {
        Self {
            uid_map: HashMap::new(),
            id_map: HashMap::new(),
            instance_counts: HashMap::new(),
            next_uid: UId(0),
        }
    }

    /// Registers the [`SubjectId`] with an intial instance count of 1
    /// if it doesn't exist yet, otherwise, increase the existing
    /// instance count.
    ///
    /// Returns the [`UId`] of the associated [`SubjectId`].
    pub fn register_instance(&mut self, id: I) -> UId {
        let uid = *self.uid_map.entry(id).or_insert_with(|| {
            self.next_uid.0 += 1;
            self.id_map.insert(self.next_uid, id);
            self.instance_counts.insert(self.next_uid, 1);
            self.next_uid
        });

        // SAFETY: `uid_counts` is added for every new UId!
        *self.instance_counts.get_mut(&uid).unwrap() += 1;

        uid
    }

    /// Reduce the instance count of a [`SubjectId`] associated with
    /// the provided [`UId`]. When the instance count reaches 0, the
    /// entire registry will be erased.
    ///
    /// Returns `true` if the instance is being successfully removed,
    /// `false` if the registry doesn't exist in the first place.
    pub fn remove_instance(&mut self, uid: &UId) -> bool {
        let Some(count) = self.instance_counts.get_mut(uid) else {
            return false;
        };

        *count -= 1;

        // Remove the underlying data when it's the last instance.
        if *count == 0 {
            let id = self.id_map.get(uid).unwrap();
            self.uid_map.remove(id);
            self.id_map.remove(uid);
            self.instance_counts.remove(uid);
        }

        true
    }

    /// Checks if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.uid_map.is_empty()
    }

    pub fn get_uid(&self, id: &I) -> Option<&UId> {
        self.uid_map.get(id)
    }

    pub fn get_id(&self, uid: &UId) -> Option<&I> {
        self.id_map.get(uid)
    }
}

impl<I: SubjectId> Default for IdRegistry<I> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct UntypedSubjectId {
    pub type_id: TypeId,
    pub uid: UId,
}

impl UntypedSubjectId {
    pub fn placeholder() -> Self {
        Self::placeholder_with_u64(0)
    }

    pub fn placeholder_with_u64(id: u64) -> Self {
        Self {
            type_id: TypeId::of::<()>(),
            uid: UId(id),
        }
    }

    pub fn new<I: SubjectId>(uid: UId) -> Self {
        Self {
            type_id: TypeId::of::<I>(),
            uid,
        }
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
pub struct ActionKey {
    /// The subject Id of the action.
    pub subject_id: UntypedSubjectId,
    /// The source and target field related to the subject.
    pub field: UntypedField,
}

#[derive(Component, Debug, Clone, Copy)]
#[component(immutable, on_remove = on_remove_id_type::<I>)]
pub struct IdType<I: SubjectId>(PhantomData<I>);

impl<I: SubjectId> IdType<I> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<I: SubjectId> Default for IdType<I> {
    fn default() -> Self {
        Self::new()
    }
}

/// Remove an instance of the target [`SubjectId`] when an action
/// entity is being despawned.
fn on_remove_id_type<I: SubjectId>(
    mut world: DeferredWorld<'_>,
    ctx: HookContext,
) {
    let uid = world
        .entity(ctx.entity)
        .get::<ActionKey>()
        .expect("Should have an `ActionKey`!")
        .subject_id
        .uid;

    let mut registry = world.resource_mut::<IdRegistry<I>>();
    registry.remove_instance(&uid);

    if registry.is_empty() {
        world.commands().remove_resource::<IdRegistry<I>>();
    }
}

#[derive(Default)]
pub struct ActionWorld {
    world: World,
}

impl ActionWorld {
    pub fn new() -> Self {
        Self {
            world: World::new(),
        }
    }

    pub fn add<I, T>(
        &mut self,
        target: I,
        field: impl Into<UntypedField>,
        action: impl Action<T>,
    ) -> ActionBuilder<'_, T>
    where
        I: SubjectId,
        T: ThreadSafe,
    {
        let field = field.into();

        let uid = self
            .world
            .get_resource_or_insert_with(|| IdRegistry::new())
            .register_instance(target);

        let key = ActionKey {
            subject_id: UntypedSubjectId::new::<I>(uid),
            field,
        };
        let world = self.world.spawn((
            key,
            IdType::<I>::new(),
            ActionStorage::new(action),
        ));

        ActionBuilder {
            world,
            key,
            _phantom: PhantomData,
        }
    }

    pub fn remove(&mut self, id: ActionId) -> Option<ActionKey> {
        let entity = id.entity();

        let key = *self
            .world
            .get_entity(entity)
            .ok()?
            .get::<ActionKey>()
            .expect("All actions should have an `ActionKey`!");

        self.world.despawn(id.entity());
        // Apply associated commands from hooks and observer when
        // despawning.
        self.world.flush();

        Some(key)
    }

    pub fn get_action<T: ThreadSafe>(
        &self,
        id: ActionId,
    ) -> Option<&impl Action<T>> {
        self.world
            .get::<ActionStorage<T>>(id.entity())
            .map(|a| &a.action)
    }

    pub fn get_id<I: SubjectId>(&self, uid: &UId) -> Option<&I> {
        self.world.get_resource::<IdRegistry<I>>()?.get_id(uid)
    }
}

impl ActionWorld {
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

pub struct ActionBuilder<'w, T> {
    world: EntityWorldMut<'w>,
    key: ActionKey,
    _phantom: PhantomData<T>,
}

/// A builder struct to insert an interpolation method for the action
/// before compiling into an [`InterpActionBuilder`].
impl<T> ActionBuilder<'_, T> {
    /// Get the [`ActionId`] of the containing action.
    pub fn id(&self) -> ActionId {
        ActionId::new(self.world.id())
    }
}

impl<'w, T> ActionBuilder<'w, T>
where
    T: 'static,
{
    /// Set the interpolation method of the action.
    pub fn with_interp(
        mut self,
        interp: InterpFn<T>,
    ) -> InterpActionBuilder<'w, T> {
        self.world.insert(InterpStorage(interp));
        InterpActionBuilder { inner: self }
    }
}

/// An action builder that has interpolation added. This builder
/// exposes more customizations for the action and allows it to be
/// compiled into a [`TrackFragment`].
pub struct InterpActionBuilder<'w, T> {
    inner: ActionBuilder<'w, T>,
}

impl<T> InterpActionBuilder<'_, T> {
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
    pub fn play(self, duration: f32) -> TrackFragment {
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
