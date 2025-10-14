use bevy_ecs::prelude::*;
use bevy_utils::prelude::*;

use crate::{
    ease::{quad, EaseFn},
    sequence::Sequence,
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

#[derive(Clone)]
pub(crate) struct ActionMeta {
    /// Target `Entity` for `Action`.
    action_id: Entity,
    /// Time at which animation should begin.
    pub(crate) start_time: f32,
    /// Duration of animation in seconds.
    pub(crate) duration: f32,
    /// Easing function to be used for animation.
    pub(crate) ease_fn: EaseFn,
}

impl ActionMeta {
    pub fn new(action_id: Entity) -> Self {
        Self {
            action_id,
            start_time: 0.0,
            duration: 0.0,
            ease_fn: quad::ease_in_out,
        }
    }

    pub fn id(&self) -> Entity {
        self.action_id
    }

    #[inline]
    pub fn end_time(&self) -> f32 {
        self.start_time + self.duration
    }
}

pub struct ActionBuilder<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
}

impl<'a, 'w, 's> ActionBuilder<'a, 'w, 's> {
    pub fn new(commands: &'a mut Commands<'w, 's>) -> Self {
        Self { commands }
    }

    pub fn play(
        &mut self,
        action: Action<impl Component, impl Send + Sync + 'static, impl Resource>,
        duration: f32,
    ) -> Sequence {
        let action_id: Entity = self.commands.spawn(action).id();
        let mut action_meta: ActionMeta = ActionMeta::new(action_id);
        action_meta.duration = duration;

        Sequence::single(action_meta)
    }

    pub fn sleep(&mut self, duration: f32) -> Sequence {
        Sequence {
            duration,
            ..default()
        }
    }
}
