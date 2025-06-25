use std::marker::PhantomData;
use std::sync::Arc;

use bevy::prelude::*;

use crate::field::Field;
use crate::sequence::Sequence;
use crate::ThreadSafe;

/// Function for interpolating a type based on a [`f32`] time.
pub type InterpFn<T> = fn(start: &T, end: &T, t: f32) -> T;

/// Easing function.
pub type EaseFn = fn(t: f32) -> f32;

/// Function for getting the target value based on an intial value.
pub trait ActionFn<T>: Fn(&T) -> T + ThreadSafe {}

impl<T, U> ActionFn<T> for U where U: Fn(&T) -> T + ThreadSafe {}

/// [`Action`]s that are related to this entity.
#[derive(Component, Reflect, Deref, Clone)]
#[reflect(Component)]
#[relationship_target(relationship = ActionTarget, linked_spawn)]
pub struct RelatedActions(Vec<Entity>);

/// The target entity that this [`Action`] belongs to.
#[derive(
    Component, Reflect, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
#[reflect(Component)]
#[relationship(relationship_target = RelatedActions)]
pub struct ActionTarget(Entity);

#[derive(Component, Reflect, Deref, DerefMut, Clone)]
#[reflect(Component)]
pub struct Action<Target>(pub Arc<dyn ActionFn<Target>>);

impl<Target> Action<Target> {
    pub fn new(target: impl ActionFn<Target>) -> Self {
        Self(Arc::new(target))
    }
}

/// A custom interpolation function for the [`Action`].
#[derive(Component, Reflect, Deref, Debug, Clone, Copy)]
#[component(immutable)]
#[reflect(Component)]
pub struct Interp<Target>(pub InterpFn<Target>);

#[derive(Component, Deref, Debug, Clone, Copy)]
#[component(immutable)]
pub struct Ease(pub EaseFn);

#[derive(Clone, Copy)]
pub struct ActionSpan {
    /// Target [`Entity`] with the [`Action`] component.
    action_id: Entity,
    /// Duration of animation in seconds.
    duration: f32,
    /// Time at which animation should begin.
    start_time: f32,
    /// Slide that this action belongs to.
    slide_index: u32,
}

impl ActionSpan {
    pub(crate) fn new(action_id: Entity, duration: f32) -> Self {
        Self {
            action_id,
            duration,
            start_time: 0.0,
            slide_index: 0,
        }
    }

    #[inline]
    pub(crate) fn with_start_time(mut self, start_time: f32) -> Self {
        self.start_time = start_time;
        self
    }

    #[inline]
    pub(crate) fn set_slide_index(&mut self, slide_index: u32) {
        self.slide_index = slide_index;
    }

    /// Target [`Entity`] with the [`Action`] component.
    #[inline]
    pub fn action_id(&self) -> Entity {
        self.action_id
    }

    #[inline]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    #[inline]
    pub fn start_time(&self) -> f32 {
        self.start_time
    }

    #[inline]
    pub fn slide_index(&self) -> u32 {
        self.slide_index
    }

    #[inline]
    pub fn end_time(&self) -> f32 {
        self.start_time + self.duration
    }
}

/// A wrapper around [`EntityCommands`] with additional methods
/// to customize the action and generate a [`Sequence`].
pub struct ActionBuilder<'w, Target> {
    action_cmd: EntityCommands<'w>,
    _marker: PhantomData<Target>,
}

impl<'w, Target> ActionBuilder<'w, Target>
where
    Target: ThreadSafe,
{
    pub fn new(action_cmd: EntityCommands<'w>) -> Self {
        Self {
            action_cmd,
            _marker: PhantomData,
        }
    }
    pub fn with_ease(&'w mut self, ease: EaseFn) -> Self {
        Self::new(self.action_cmd.insert(Ease(ease)).reborrow())
    }

    pub fn with_interp(
        &'w mut self,
        interp: InterpFn<Target>,
    ) -> Self {
        Self::new(self.action_cmd.insert(Interp(interp)).reborrow())
    }

    pub fn play(&mut self, duration: f32) -> Sequence {
        Sequence::single(ActionSpan::new(
            self.action_cmd.id(),
            duration,
        ))
    }
}

pub trait ActionBuilderExt<'w> {
    fn act<Source, Target>(
        &'w mut self,
        field: Field<Source, Target>,
        action: impl ActionFn<Target>,
    ) -> ActionBuilder<'w, Source>
    where
        Source: ThreadSafe,
        Target: ThreadSafe;
}

impl<'w> ActionBuilderExt<'w> for EntityCommands<'w> {
    fn act<Source, Target>(
        &'w mut self,
        field: Field<Source, Target>,
        action: impl ActionFn<Target>,
    ) -> ActionBuilder<'w, Source>
    where
        Source: ThreadSafe,
        Target: ThreadSafe,
    {
        let action_target = ActionTarget(self.id());
        ActionBuilder::new(self.commands_mut().spawn((
            action_target,
            field.to_hash(),
            Action::new(action),
        )))
    }
}

#[cfg(test)]
mod test {
    use crate::field::field;

    use super::*;

    #[test]
    fn test_action_builder() {
        const DURATION: f32 = 2.0;

        let action_fn = |x: &f32| x + 3.0;

        let mut world = World::new();

        let seq = world
            .commands()
            .spawn(Transform::default())
            .act(field!(<Transform>::translation::x), action_fn)
            .with_ease(|t| t * t)
            .play(DURATION);

        world.flush();

        assert_eq!(seq.spans.len(), 1);
        assert_eq!(seq.spans[0].duration, DURATION);
        assert_eq!(seq.duration(), DURATION);
        // 1 for the action entity, 1 for the original entity.
        assert_eq!(world.entities().len(), 2);

        // Only 1 action is being created.
        let action =
            world.query::<&Action<f32>>().single(&world).unwrap();

        assert_eq!(action(&2.0), action_fn(&2.0));
    }
}
