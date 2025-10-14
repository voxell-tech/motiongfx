use bevy_ecs::prelude::*;

use crate::{
    ease::{quad, EaseFn},
    sequence::{sequence_controller_interp, Sequence},
};

pub type InterpFn<CompType, InterpType, ResType> = fn(
    component: &mut CompType,
    begin: &InterpType,
    end: &InterpType,
    t: f32,
    resource: &mut ResMut<ResType>,
);

/// Basic data structure to describe an animation action.
#[derive(Component, Clone, Copy)]
pub struct Action<CompType, InterpType, ResType>
where
    CompType: Component,
    InterpType: Send + Sync + 'static,
    ResType: Resource,
{
    /// Target `Entity` for `Component` manipulation.
    pub(crate) target_id: Entity,
    /// Initial state of the animation.
    pub(crate) begin: InterpType,
    /// Final state of the animation.
    pub(crate) end: InterpType,
    /// Interpolation function to be used for animation.
    pub(crate) interp_fn: InterpFn<CompType, InterpType, ResType>,
}

impl<CompType, InterpType, ResType> Action<CompType, InterpType, ResType>
where
    CompType: Component,
    InterpType: Send + Sync + 'static,
    ResType: Resource,
{
    pub fn new(
        target_id: Entity,
        begin: InterpType,
        end: InterpType,
        interp_fn: InterpFn<CompType, InterpType, ResType>,
    ) -> Self {
        Self {
            target_id,
            begin,
            end,
            interp_fn,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ActionMeta {
    /// Target `Entity` for `Action`.
    action_id: Entity,
    /// Time at which animation should begin.
    pub(crate) start_time: f32,
    /// Duration of animation in seconds.
    pub(crate) duration: f32,
    /// Easing function to be used for animation.
    pub(crate) ease_fn: EaseFn,
    /// Slide that this action belongs to.
    pub(crate) slide_index: usize,
}

impl ActionMeta {
    pub fn new(action_id: Entity) -> Self {
        Self {
            action_id,
            start_time: 0.0,
            duration: 0.0,
            ease_fn: quad::ease_in_out,
            slide_index: 0,
        }
    }

    pub fn id(&self) -> Entity {
        self.action_id
    }

    #[inline]
    pub fn with_start_time(mut self, start_time: f32) -> Self {
        self.start_time = start_time;
        self
    }

    #[inline]
    pub fn end_time(&self) -> f32 {
        self.start_time + self.duration
    }
}

pub trait ActionBuilder {
    fn play(
        &mut self,
        action: Action<impl Component, impl Send + Sync + 'static, impl Resource>,
        duration: f32,
    ) -> Sequence;
    fn play_sequence(
        &mut self,
        target_id: Entity,
        begin: f32,
        end: f32,
        playback_speed: f32,
    ) -> Sequence;
    fn sleep(&mut self, duration: f32) -> Sequence;
}

impl ActionBuilder for Commands<'_, '_> {
    fn play(
        &mut self,
        action: Action<impl Component, impl Send + Sync + 'static, impl Resource>,
        duration: f32,
    ) -> Sequence {
        let action_id = self.spawn(action).id();
        let mut action_meta = ActionMeta::new(action_id);
        action_meta.duration = duration;

        // TODO: create single sequence
        Sequence::single(action_meta)
    }

    fn play_sequence(
        &mut self,
        target_id: Entity,
        begin: f32,
        end: f32,
        playback_speed: f32,
    ) -> Sequence {
        let action = Action::new(target_id, begin, end, sequence_controller_interp);

        let action_id = self.spawn(action).id();
        let mut action_meta = ActionMeta::new(action_id);

        // Prevent division by 0.0
        if f32::abs(playback_speed) <= f32::EPSILON {
            action_meta.duration = 0.0;
        } else {
            action_meta.duration = f32::abs(end - begin) / playback_speed;
        }

        Sequence::single(action_meta)
    }

    fn sleep(&mut self, duration: f32) -> Sequence {
        Sequence::empty(duration)
    }
}
