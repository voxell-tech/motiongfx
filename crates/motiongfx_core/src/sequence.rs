use std::iter::Iterator;

use bevy::prelude::*;
use bevy::{asset::AsAssetId, ecs::component::Mutable};

use crate::action::{Action, ActionMeta};
use crate::ThreadSafe;

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
    // TODO(perf): Use SmallVec to prevent heap allocations for single action sequences.
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
    time: f32,
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

// SEQUENCE ORDERING FUNCTIONS

pub trait MultiSeqOrd {
    /// Run one [`Sequence`] after another.
    fn chain(self) -> Sequence;
    /// Run all [`Sequence`]s concurrently and wait for all of them to finish.
    fn all(self) -> Sequence;
    /// Run all [`Sequence`]s concurrently and wait for any of them to finish.
    fn any(self) -> Sequence;
    /// Run one [`Sequence`] after another with a fixed delay time.
    fn flow(self, delay: f32) -> Sequence;
}

impl MultiSeqOrd for &[Sequence] {
    fn chain(self) -> Sequence {
        chain(self)
    }

    fn all(self) -> Sequence {
        all(self)
    }

    fn any(self) -> Sequence {
        any(self)
    }

    fn flow(self, t: f32) -> Sequence {
        flow(t, self)
    }
}

pub trait SingleSeqOrd {
    /// Run a [`Sequence`] after a fixed delay time.
    fn delay(self, t: f32) -> Sequence;
}

impl SingleSeqOrd for Sequence {
    fn delay(self, t: f32) -> Sequence {
        delay(t, self)
    }
}

/// Run one [`Sequence`] after another.
pub fn chain(sequences: &[Sequence]) -> Sequence {
    let mut final_sequence = Sequence::default();
    let mut chain_duration = 0.0;

    for sequence in sequences {
        for action_meta in &sequence.action_metas {
            final_sequence.action_metas.push(
                action_meta.with_start_time(
                    action_meta.start_time + chain_duration,
                ),
            );
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
pub fn flow(t: f32, sequences: &[Sequence]) -> Sequence {
    let mut final_sequence = Sequence::default();
    let mut flow_duration = 0.0;
    let mut final_duration = 0.0;

    for sequence in sequences {
        for action_meta in &sequence.action_metas {
            final_sequence.action_metas.push(
                action_meta.with_start_time(
                    action_meta.start_time + flow_duration,
                ),
            );
        }

        flow_duration += t;
        final_duration = f32::max(
            final_duration,
            flow_duration + sequence.duration,
        );
    }

    final_sequence.duration = final_duration;
    final_sequence
}

/// Run a [`Sequence`] after a fixed delay time.
pub fn delay(t: f32, sequence: Sequence) -> Sequence {
    let mut final_sequence = Sequence::default();

    for action_meta in &sequence.action_metas {
        final_sequence.action_metas.push(
            action_meta.with_start_time(action_meta.start_time + t),
        );
    }

    final_sequence.duration = sequence.duration + t;
    final_sequence
}

/// System for animating the [`Component`] related [`Action`]s that are inside the [`Sequence`].
pub fn animate_component<Comp, Target>(
    mut q_components: Query<&mut Comp>,
    q_actions: Query<&'static Action<Target, Comp>>,
    q_sequences: Query<(&Sequence, &SequenceController)>,
) where
    Comp: Component<Mutability = Mutable>,
    Target: ThreadSafe,
{
    for (sequence, sequence_controller) in q_sequences.iter() {
        if let Some(action) = generate_action_iter(
            &q_actions,
            sequence,
            sequence_controller,
        ) {
            for (
                Action {
                    entity,
                    start,
                    end,
                    get_field_fn,
                    interp_fn,
                    ease_fn,
                },
                action_meta,
            ) in action
            {
                // Get component to mutate based on action id
                let Ok(mut component) = q_components.get_mut(*entity)
                else {
                    continue;
                };

                let mut unit_time = (sequence_controller.target_time
                    - action_meta.start_time)
                    / action_meta.duration;

                // In case of division by 0.0
                if f32::is_nan(unit_time) {
                    unit_time = 0.0;
                }

                unit_time = f32::clamp(unit_time, 0.0, 1.0);
                // Calculate unit time using ease function
                unit_time = ease_fn(unit_time);

                // Mutate the component using interpolate function
                let field = get_field_fn(&mut component);
                *field = interp_fn(start, end, unit_time);
            }
        }
    }
}

/// System for animating the [`Asset`] related [`Action`]s that are inside the [`Sequence`].
pub fn animate_asset<Comp, Target>(
    q_handles: Query<&Comp>,
    mut assets: ResMut<Assets<Comp::Asset>>,
    q_actions: Query<&'static Action<Target, Comp::Asset>>,
    q_sequences: Query<(&Sequence, &SequenceController)>,
) where
    Comp: Component + AsAssetId,
    Target: ThreadSafe,
{
    // let q_handles = q_handles.iter
    for (sequence, sequence_controller) in q_sequences.iter() {
        if let Some(action) = generate_action_iter(
            &q_actions,
            sequence,
            sequence_controller,
        ) {
            for (
                Action {
                    entity,
                    start,
                    end,
                    get_field_fn,
                    interp_fn,
                    ease_fn,
                },
                action_meta,
            ) in action
            {
                // Get handle based on action id
                let Ok(handle) = q_handles.get(*entity) else {
                    continue;
                };

                // Get asset to mutate based on the handle id
                let Some(asset) =
                    assets.get_mut(handle.as_asset_id())
                else {
                    continue;
                };

                let mut unit_time = (sequence_controller.target_time
                    - action_meta.start_time)
                    / action_meta.duration;

                // In case of division by 0.0
                if f32::is_nan(unit_time) {
                    unit_time = 0.0;
                }

                unit_time = f32::clamp(unit_time, 0.0, 1.0);
                // Calculate unit time using ease function
                unit_time = ease_fn(unit_time);

                // Mutate the component using interpolate function
                let field = get_field_fn(asset);
                *field = interp_fn(start, end, unit_time);
            }
        }
    }
}

/// Update [`SequenceController::target_time`] based on [`SequencePlayer::time_scale`].
pub(crate) fn update_target_time(
    mut q_sequences: Query<(
        &Sequence,
        &mut SequenceController,
        &SequencePlayer,
    )>,
    time: Res<Time>,
) {
    for (sequence, mut sequence_controller, sequence_player) in
        q_sequences.iter_mut()
    {
        sequence_controller.target_time = f32::clamp(
            sequence_controller.target_time
                + time.delta_secs() * sequence_player.time_scale,
            0.0,
            sequence.duration(),
        );
    }
}

/// Safely update [`SequenceController::time`] after performing
/// all the necessary actions.
pub(crate) fn update_time(
    mut q_sequences: Query<(&Sequence, &mut SequenceController)>,
) {
    for (sequence, mut controller) in q_sequences.iter_mut() {
        controller.target_time = f32::clamp(
            controller.target_time,
            0.0,
            sequence.duration(),
        );

        controller.time = controller.target_time;
    }
}

fn generate_action_iter<'a, T, U>(
    q_actions: &'a Query<&'static Action<T, U>>,
    sequence: &'a Sequence,
    controller: &'a SequenceController,
) -> Option<impl Iterator<Item = (&'a Action<T, U>, &'a ActionMeta)>>
where
    T: ThreadSafe,
{
    // Do not perform any actions if there are no changes to the timeline timings
    // or there are no actions at all.
    if controller.time == controller.target_time
        || sequence.action_metas.is_empty()
    {
        return None;
    }

    // Calculate time flow direction based on time difference
    let direction =
        f32::signum(controller.target_time - controller.time)
            as isize;

    let timeline_start =
        f32::min(controller.time, controller.target_time);
    let timeline_end =
        f32::max(controller.time, controller.target_time);

    let mut start_index = 0;
    let mut end_index = sequence.action_metas.len() - 1;

    // Swap direction if needed
    if direction == -1 {
        start_index = end_index;
        end_index = 0;
    }

    let mut action_index = start_index;

    Some(std::iter::from_fn(move || {
        // Loop through `Action`s in the direction that the timeline is going towards.
        loop {
            if action_index
                == (end_index as isize + direction) as usize
            {
                return None;
            }

            let action_meta = &sequence.action_metas[action_index];
            let action_id = action_meta.id();

            let slide_direction = isize::signum(
                controller.target_slide_index as isize
                    - action_meta.slide_index as isize,
            );

            // Continue only when slide direction matches or is 0
            if slide_direction != 0 && slide_direction != direction {
                return None;
            }

            action_index =
                (action_index as isize + direction) as usize;

            let is_time_overlap = time_range_overlap(
                action_meta.start_time,
                action_meta.end_time(),
                timeline_start,
                timeline_end,
            );
            // Ignore if `ActionMeta` not in range
            if is_time_overlap == false {
                continue;
            }

            // Ignore if `Action` does not exists
            if let Ok(action) = q_actions.get(action_id) {
                return Some((action, action_meta));
            }
        }
    }))
}

/// Calculate if 2 time range (in float) overlaps.
fn time_range_overlap(
    a_begin: f32,
    a_end: f32,
    b_begin: f32,
    b_end: f32,
) -> bool {
    a_begin <= b_end && b_begin <= a_end
}
