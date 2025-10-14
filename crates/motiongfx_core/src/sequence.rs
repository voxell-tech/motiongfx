use crate::{
    action::{Action, ActionMeta, ActionMetaGroup},
    timeline::Timeline,
};
use bevy_ecs::prelude::*;

/// An array of `Action`.
#[derive(Resource)]
pub struct Sequence {
    duration: f32,
    action_metas: Vec<ActionMeta>,
}

impl Sequence {
    pub fn new() -> Self {
        Sequence {
            duration: 0.0,
            action_metas: Vec::new(),
        }
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn play(&mut self, action_grp: ActionMetaGroup) {
        let mut max_duration: f32 = 0.0;

        for action_meta in action_grp.action_metas {
            self.action_metas.push(
                action_meta
                    .clone()
                    .with_start_time(action_meta.start_time() + self.duration),
            );

            max_duration = f32::max(
                max_duration,
                action_meta.start_time() + action_meta.duration(),
            );
        }

        self.duration = max_duration;
    }
}

// ANIMATION FLOW FUNCTIONS

/// Run one action after another.
pub fn chain(action_grps: &[ActionMetaGroup]) -> ActionMetaGroup {
    let mut final_action_grp: ActionMetaGroup = ActionMetaGroup::new();
    let mut chain_duration: f32 = 0.0;

    for action_grp in action_grps {
        for action_meta in &action_grp.action_metas {
            final_action_grp.action_metas.push(
                action_meta
                    .clone()
                    .with_start_time(action_meta.start_time() + chain_duration),
            );
        }

        chain_duration += action_grp.duration;
    }

    final_action_grp.duration += chain_duration;
    final_action_grp
}

/// Run all actions concurrently and wait for all of them to finish.
pub fn all(action_grps: &[ActionMetaGroup]) -> ActionMetaGroup {
    let mut final_action_grp: ActionMetaGroup = ActionMetaGroup::new();
    let mut max_duration: f32 = 0.0;

    for action_grp in action_grps {
        for action_meta in &action_grp.action_metas {
            final_action_grp.action_metas.push(action_meta.clone());
        }

        max_duration = f32::max(max_duration, action_grp.duration);
    }

    final_action_grp.duration = max_duration;
    final_action_grp
}

/// Run all actions concurrently and wait for any of them to finish.
pub fn any(action_grps: &[ActionMetaGroup]) -> ActionMetaGroup {
    let mut final_action_grp: ActionMetaGroup = ActionMetaGroup::new();
    let mut min_duration: f32 = 0.0;

    for action_grp in action_grps {
        for action_meta in &action_grp.action_metas {
            final_action_grp.action_metas.push(action_meta.clone());
        }

        min_duration = f32::min(min_duration, action_grp.duration);
    }

    final_action_grp.duration = min_duration;
    final_action_grp
}

pub fn flow(delay: f32, action_grps: &[ActionMetaGroup]) -> ActionMetaGroup {
    let mut final_action_grp: ActionMetaGroup = ActionMetaGroup::new();
    let mut flow_duration: f32 = 0.0;
    let mut final_duration: f32 = 0.0;

    for action_grp in action_grps {
        for action_meta in &action_grp.action_metas {
            final_action_grp.action_metas.push(
                action_meta
                    .clone()
                    .with_start_time(action_meta.start_time() + flow_duration),
            );
        }

        flow_duration += delay;
        final_duration = f32::max(final_duration, flow_duration + action_grp.duration);
    }

    final_action_grp.duration = final_duration;
    final_action_grp
}

pub fn delay(delay: f32, action_grp: ActionMetaGroup) -> ActionMetaGroup {
    let mut final_action_grp: ActionMetaGroup = ActionMetaGroup::new();

    for action_meta in &action_grp.action_metas {
        final_action_grp.action_metas.push(
            action_meta
                .clone()
                .with_start_time(action_meta.start_time() + delay),
        );
    }

    final_action_grp.duration = delay + action_grp.duration;

    final_action_grp
}

/// System for playing the `Action`s that are inside the `Sequence`.
pub fn sequence_player_system<C, T, R>(
    mut q_component: Query<&mut C>,
    q_actions: Query<&Action<C, T, R>>,
    scene: Res<Sequence>,
    timeline: Res<Timeline>,
    mut resource: ResMut<R>,
) where
    C: Component,
    T: Send + Sync + 'static,
    R: Resource,
{
    // Do not perform any actions if there are no changes to the timeline timings.
    if timeline.curr_time == timeline.target_time {
        return;
    }

    let direction: i32 = f32::signum(timeline.target_time - timeline.curr_time) as i32;

    let timeline_start: f32 = f32::min(timeline.curr_time, timeline.target_time);
    let timeline_end: f32 = f32::max(timeline.curr_time, timeline.target_time);

    let mut start_index: usize = 0;
    let mut end_index: usize = scene.action_metas.len() - 1;

    // Swap direction if needed
    if direction == -1 {
        start_index = end_index;
        end_index = 0;
    }

    let mut action_index: usize = start_index;

    // Loop through `Action`s in the direction that the timeline is going towards.
    loop {
        if action_index == (end_index as i32 + direction) as usize {
            break;
        }

        let action_meta: &ActionMeta = &scene.action_metas[action_index];
        let action_id: Entity = action_meta.id();

        action_index = (action_index as i32 + direction) as usize;

        // Ignore if `ActionMeta` not in range
        if !time_range_overlap(
            action_meta.start_time(),
            action_meta.end_time(),
            timeline_start,
            timeline_end,
        ) {
            continue;
        }

        // Ignore if `Action` does not exists
        let Ok(action) = q_actions.get(action_id) else {
            continue;
        };

        // Get component to mutate based on action id
        if let Ok(mut component) = q_component.get_mut(action.target_id) {
            let mut unit_time: f32 =
                (timeline.target_time - action_meta.start_time()) / action_meta.duration();

            // In case of division by 0.0
            if f32::is_nan(unit_time) {
                unit_time = 0.0;
            }

            unit_time = f32::clamp(unit_time, 0.0, 1.0);
            // Calculate unit time using ease function
            unit_time = (action_meta.ease_fn)(unit_time);

            // Mutate the component using interpolate function
            (action.interp_fn)(
                &mut component,
                &action.begin,
                &action.end,
                unit_time,
                &mut resource,
            );
        }
    }
}

/// Calculate if 2 time range (in float) overlaps.
fn time_range_overlap(a_begin: f32, a_end: f32, b_begin: f32, b_end: f32) -> bool {
    a_begin <= b_end && b_begin <= a_end
}
