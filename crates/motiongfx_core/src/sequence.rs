use bevy_ecs::prelude::*;
use bevy_time::prelude::*;
use bevy_utils::prelude::*;

use crate::{
    action::{Action, ActionMeta},
    ease::EaseFn,
    lerp::*,
    EmptyRes,
};

/// Bundle to encapsulate [`Sequence`] and [`SequenceController`].
#[derive(Bundle, Default)]
pub struct SequenceBundle {
    pub sequence: Sequence,
    pub sequence_controller: SequenceController,
}

impl SequenceBundle {
    pub fn from_sequence(sequence: Sequence) -> Self {
        Self {
            sequence,
            ..default()
        }
    }
}

/// Bundle to encapsulate [`Sequence`], [`SequenceController`], and [`SequencePlayer`].
#[derive(Bundle, Default)]
pub struct SequencePlayerBundle {
    pub sequence: Sequence,
    pub sequence_controller: SequenceController,
    pub sequence_player: SequencePlayer,
}

impl SequencePlayerBundle {
    pub fn from_sequence(sequence: Sequence) -> Self {
        Self {
            sequence,
            ..default()
        }
    }
}

/// A group of actions in chronological order.
#[derive(Component, Default, Clone)]
pub struct Sequence {
    duration: f32,
    pub(crate) action_metas: Vec<ActionMeta>,
}

impl Sequence {
    pub(crate) fn single(action_meta: ActionMeta) -> Self {
        let duration = action_meta.duration;
        Self {
            action_metas: vec![action_meta],
            duration,
        }
    }

    pub(crate) fn empty(duration: f32) -> Self {
        Self {
            duration,
            ..default()
        }
    }

    /// Add easing to all the [`Action`]s within this [`Sequence`].
    pub fn with_ease(mut self, ease_fn: EaseFn) -> Self {
        for action_meta in &mut self.action_metas {
            action_meta.ease_fn = ease_fn;
        }

        self
    }

    pub(crate) fn set_slide_index(&mut self, slide_index: usize) {
        for action_meta in &mut self.action_metas {
            action_meta.slide_index = slide_index;
        }
    }

    #[inline]
    pub fn duration(&self) -> f32 {
        self.duration
    }
}

/// Plays the [`Sequence`] component attached to this entity through `target_time` manipulation.
#[derive(Component, Default)]
pub struct SequenceController {
    pub(crate) curr_time: f32,
    /// Target time to reach (and not exceed).
    pub target_time: f32,
    /// Target slide index to reach (and not exceed).
    pub target_slide_index: usize,
}

/// Manipulates the `target_time` variable of the [`SequenceController`] component attached to this entity with a `time_scale`.
#[derive(Component, Default)]
pub struct SequencePlayer {
    pub time_scale: f32,
}

/// Interpolation for [`SequenceController`].
pub(crate) fn sequence_controller_interp(
    player: &mut SequenceController,
    begin: &f32,
    end: &f32,
    t: f32,
    _: &mut ResMut<EmptyRes>,
) {
    player.target_time = f32::lerp(begin, end, t);
}

// ANIMATION FLOW FUNCTIONS

#[macro_export]
macro_rules! any {
    (&$motion:expr) => {
        $crate::sequence::any(&$motion)
    };
    ($($motion:expr),+ $(,)?) => {
        $crate::sequence::any(&[$($motion),+])
    };
}

#[macro_export]
macro_rules! chain {
    (&$motion:expr) => {
        $crate::sequence::chain(&$motion)
    };
    ($($motion:expr),+ $(,)?) => {
        $crate::sequence::chain(&[$($motion),+])
    };
}

#[macro_export]
macro_rules! all {
    (&$motion:expr) => {
       $crate::sequence::all(&$motion)
    };
    ($($motion:expr),+ $(,)?) => {
        $crate::sequence::all(&[$($motion),+])
    };
}

#[macro_export]
macro_rules! flow {
    ($duration:expr, &$motion:expr) => {
        $crate::sequence::flow($duration, &$motion)
    };
    ($duration:expr, $($motion:expr),+ $(,)?) => {
        $crate::sequence::flow($duration, &[$($motion),+])
    };
}

#[macro_export]
macro_rules! delay {
    ($duration:expr, &$motion:expr) => {
        $crate::sequence::delay($duration, &$motion)
    };
}

/// Run one [`Sequence`] after another.
pub fn chain(sequences: &[Sequence]) -> Sequence {
    let mut final_sequence = Sequence::default();
    let mut chain_duration = 0.0;

    for sequence in sequences {
        for action_meta in &sequence.action_metas {
            final_sequence
                .action_metas
                .push(action_meta.with_start_time(action_meta.start_time + chain_duration));
        }

        chain_duration += sequence.duration;
    }

    final_sequence.duration = chain_duration;
    final_sequence
}

/// Run all [`Sequence`]s concurrently and wait for all of them to finish.
pub fn all(sequences: &[Sequence]) -> Sequence {
    let mut final_sequence = Sequence::default();
    let mut max_duration = 0.0;

    for sequence in sequences {
        for action_meta in &sequence.action_metas {
            final_sequence.action_metas.push(*action_meta);
        }

        max_duration = f32::max(max_duration, sequence.duration);
    }

    final_sequence.duration = max_duration;
    final_sequence
}

/// Run all [`Sequence`]s concurrently and wait for any of them to finish.
pub fn any(sequences: &[Sequence]) -> Sequence {
    let mut final_sequence = Sequence::default();
    let mut min_duration = 0.0;

    for action_grp in sequences {
        for action_meta in &action_grp.action_metas {
            final_sequence.action_metas.push(*action_meta);
        }

        min_duration = f32::min(min_duration, action_grp.duration);
    }

    final_sequence.duration = min_duration;
    final_sequence
}

/// Run one [`Sequence`] after another with a fixed delay time.
pub fn flow(delay: f32, sequences: &[Sequence]) -> Sequence {
    let mut final_sequence = Sequence::default();
    let mut flow_duration = 0.0;
    let mut final_duration = 0.0;

    for sequence in sequences {
        for action_meta in &sequence.action_metas {
            final_sequence
                .action_metas
                .push(action_meta.with_start_time(action_meta.start_time + flow_duration));
        }

        flow_duration += delay;
        final_duration = f32::max(final_duration, flow_duration + sequence.duration);
    }

    final_sequence.duration = final_duration;
    final_sequence
}

/// Run an [`Sequence`] after a fixed delay time.
pub fn delay(delay: f32, sequence: &Sequence) -> Sequence {
    let mut final_sequence = Sequence::default();

    for action_meta in &sequence.action_metas {
        final_sequence
            .action_metas
            .push(action_meta.with_start_time(action_meta.start_time + delay));
    }

    final_sequence.duration = sequence.duration + delay;
    final_sequence
}

/// System for playing the [`Action`]s that are inside the [`Sequence`].
pub fn sequence_update_system<CompType, InterpType, ResType>(
    mut q_components: Query<&mut CompType>,
    q_actions: Query<&Action<CompType, InterpType, ResType>>,
    q_sequences: Query<(&Sequence, &SequenceController)>,
    mut resource: ResMut<ResType>,
) where
    CompType: Component,
    InterpType: Send + Sync + 'static,
    ResType: Resource,
{
    for (sequence, sequence_controller) in q_sequences.iter() {
        play_sequence(
            &mut q_components,
            &q_actions,
            sequence,
            sequence_controller,
            &mut resource,
        );
    }
}

/// Safely update the `target_time` in [`SequenceController`] after performing all the necessary actions.
pub(crate) fn sequence_controller(mut q_sequences: Query<(&Sequence, &mut SequenceController)>) {
    for (sequence, mut sequence_controller) in q_sequences.iter_mut() {
        sequence_controller.target_time =
            f32::clamp(sequence_controller.target_time, 0.0, sequence.duration());
        sequence_controller.curr_time = sequence_controller.target_time;
    }
}

/// Update [`SequenceController`] based on `time_scale` of [`SequencePlayer`].
pub(crate) fn sequence_player(
    mut q_sequences: Query<(&Sequence, &mut SequenceController, &SequencePlayer)>,
    time: Res<Time>,
) {
    for (sequence, mut sequence_controller, sequence_player) in q_sequences.iter_mut() {
        sequence_controller.target_time = f32::clamp(
            sequence_controller.target_time + time.delta_seconds() * sequence_player.time_scale,
            0.0,
            sequence.duration(),
        );
    }
}

fn play_sequence<CompType, InterpType, ResType>(
    q_components: &mut Query<&mut CompType>,
    q_actions: &Query<&Action<CompType, InterpType, ResType>>,
    sequence: &Sequence,
    sequence_controller: &SequenceController,
    resource: &mut ResMut<ResType>,
) where
    CompType: Component,
    InterpType: Send + Sync + 'static,
    ResType: Resource,
{
    // Do not perform any actions if there are no changes to the timeline timings
    // or there are no actions at all.
    if sequence_controller.curr_time == sequence_controller.target_time
        || sequence.action_metas.is_empty()
    {
        return;
    }

    // Calculate time flow direction based on time difference
    let direction =
        f32::signum(sequence_controller.target_time - sequence_controller.curr_time) as isize;

    let timeline_start = f32::min(
        sequence_controller.curr_time,
        sequence_controller.target_time,
    );
    let timeline_end = f32::max(
        sequence_controller.curr_time,
        sequence_controller.target_time,
    );

    let mut start_index = 0;
    let mut end_index = sequence.action_metas.len() - 1;

    // Swap direction if needed
    if direction == -1 {
        start_index = end_index;
        end_index = 0;
    }

    let mut action_index = start_index;

    // Loop through `Action`s in the direction that the timeline is going towards.
    loop {
        if action_index == (end_index as isize + direction) as usize {
            break;
        }

        let action_meta = &sequence.action_metas[action_index];
        let action_id = action_meta.id();

        let slide_direction = isize::signum(
            sequence_controller.target_slide_index as isize - action_meta.slide_index as isize,
        );

        // Continue only when slide direction matches or is 0
        if slide_direction != 0 && slide_direction != direction {
            break;
        }

        action_index = (action_index as isize + direction) as usize;

        // Ignore if `ActionMeta` not in range
        if !crate::time_range_overlap(
            action_meta.start_time,
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
        if let Ok(mut component) = q_components.get_mut(action.target_id) {
            let mut unit_time =
                (sequence_controller.target_time - action_meta.start_time) / action_meta.duration;

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
                resource,
            );
        }
    }
}
