use bevy::prelude::*;
use smallvec::SmallVec;

use crate::sequence::{Sequence, SequenceController};
use crate::MotionGfxSet;

pub(super) struct TimelinePlugin;

impl Plugin for TimelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            update_target_time.in_set(MotionGfxSet::TargetTime),
        )
        .add_observer(jump_sequence);
    }
}

pub trait CreateTimelineAppExt {
    fn create_timeline(
        &mut self,
        sequences: impl IntoIterator<Item = Sequence>,
    ) -> EntityCommands<'_>;
}

impl CreateTimelineAppExt for Commands<'_, '_> {
    fn create_timeline(
        &mut self,
        sequences: impl IntoIterator<Item = Sequence>,
    ) -> EntityCommands<'_> {
        let timeline_id = self.spawn_empty().id();

        for sequence in sequences {
            self.spawn((sequence, TargetTimeline(timeline_id)));
        }

        self.entity(timeline_id)
    }
}

/// Update [`SequenceController::target_time`] based on [`Timeline`].
fn update_target_time(
    mut q_timelines: Query<(
        &mut Timeline,
        &mut TimelinePlayback,
        &TimeScale,
    )>,
    mut q_sequences: Query<(&Sequence, &mut SequenceController)>,
    time: Res<Time>,
) -> Result {
    for (mut timeline, mut playback, time_scale) in
        q_timelines.iter_mut()
    {
        let Some(sequence_id) = timeline.curr_sequence_id() else {
            continue;
        };

        let (sequence, mut controller) =
            q_sequences.get_mut(sequence_id)?;

        match *playback {
            TimelinePlayback::Forward => {
                if controller.curr_time() >= sequence.duration() {
                    // When playback indicates moving forward
                    // and we've reached the end.
                    timeline.sequence_point = SequencePoint::End;
                    playback.pause();
                    continue;
                } else {
                    controller.target_time +=
                        time_scale.get() * time.delta_secs();
                }
            }
            TimelinePlayback::Backward => {
                if controller.curr_time() <= 0.0 {
                    // When playback indicates moving backward
                    // and we've reached the start.
                    timeline.sequence_point = SequencePoint::Start;
                    playback.pause();
                    continue;
                } else {
                    controller.target_time -=
                        time_scale.get() * time.delta_secs();
                }
            }
            TimelinePlayback::Pause => continue,
        }

        timeline.sequence_point =
            SequencePoint::Exact(controller.target_time);
    }

    Ok(())
}

fn jump_sequence(
    trigger: Trigger<JumpSequence>,
    mut q_timelines: Query<(&mut Timeline, &mut TimelinePlayback)>,
    mut q_sequences: Query<(&Sequence, &mut SequenceController)>,
) -> Result {
    let timeline_id = trigger.target();
    let jump = trigger.event();

    let (mut timeline, mut playback) =
        q_timelines.get_mut(timeline_id)?;

    let target_index =
        jump.index.min(timeline.sequence_ids.len() - 1);

    if target_index != timeline.sequence_index() {
        // Fast-forward or rewind sequences that have been
        // skipped over to set their final state.
        let is_forward = target_index > timeline.sequence_index();
        let (mut min, mut max) = (
            target_index.min(timeline.sequence_index),
            target_index.max(timeline.sequence_index),
        );

        if is_forward == false {
            // Shift indices forward to prevent altering
            // the target sequence.
            max += 1;
            min += 1;
        }

        let set_target_time = if is_forward {
            // Set to the end if the index is moving forward.
            |sequence: &Sequence,
             controller: &mut SequenceController| {
                controller.target_time = sequence.duration();
            }
        } else {
            // Set to the start if the index is moving backward.
            |_: &Sequence, controller: &mut SequenceController| {
                controller.target_time = 0.0;
            }
        };

        for i in min..max {
            let sequence_id = timeline.sequence_ids[i];
            let (sequence, mut controller) =
                q_sequences.get_mut(sequence_id)?;

            // Set the target time based on the conditioned closure.
            set_target_time(sequence, &mut controller);
        }
    }

    // Apply jump configuration to the timeline.
    timeline.sequence_index = target_index;
    *playback = jump.playback;
    timeline.sequence_point = jump.point;

    // No sequence to play at all!
    let Some(sequence_id) = timeline.curr_sequence_id() else {
        warn!("Timeline {timeline_id} is empty!");
        return Ok(());
    };

    // Apply the waypoint to the target sequence.
    let (sequence, mut controller) =
        q_sequences.get_mut(sequence_id)?;

    match jump.point {
        SequencePoint::Start => controller.target_time = 0.0,
        SequencePoint::End => {
            controller.target_time = sequence.duration()
        }
        SequencePoint::Exact(time) => controller.target_time = time,
    }

    Ok(())
}

#[derive(Event, Debug)]
pub struct JumpSequence {
    pub index: usize,
    /// The playback state that the timeline should use after jumping
    /// to the target [`Sequence`].
    pub playback: TimelinePlayback,
    /// Deteremines the starting point of the [`Sequence`].
    pub point: SequencePoint,
}

/// Manipulates [`SequenceController::target_time`].
#[derive(Component, Debug)]
#[relationship_target(relationship = TargetTimeline, linked_spawn)]
#[require(TimelinePlayback, TimeScale)]
pub struct Timeline {
    /// The [`Sequence`]s that is related to this timeline.
    #[relationship]
    sequence_ids: SmallVec<[Entity; 1]>,
    /// The index in `sequence_ids`.
    sequence_index: usize,
    /// The point of the current sequence.
    ///
    /// Updates every frame in the [`update_target_time()`] system.
    sequence_point: SequencePoint,
}

impl Timeline {
    /// Get the current sequence id based on `sequence_index`.
    ///
    /// Returns an optional entity as `sequence_ids` might be empty.
    pub fn curr_sequence_id(&self) -> Option<Entity> {
        self.sequence_ids.get(self.sequence_index).copied()
    }

    /// Get the current sequence index.
    #[inline(always)]
    pub fn sequence_index(&self) -> usize {
        self.sequence_index
    }

    /// Get the [`SequencePoint`] of the current sequence.
    #[inline(always)]
    pub fn sequence_point(&self) -> SequencePoint {
        self.sequence_point
    }

    /// Get the number of sequences in the timeline.
    #[inline]
    pub fn sequence_len(&self) -> usize {
        self.sequence_ids.len()
    }

    /// Check if the current sequence is the last one.
    #[inline]
    pub fn is_last_sequence(&self) -> bool {
        self.sequence_index() == self.sequence_len() - 1
    }

    /// Check if the current sequence is the first one.
    #[inline]
    pub fn is_first_sequence(&self) -> bool {
        self.sequence_index() == 0
    }
}

/// The target [`Timeline`] that this [`Sequence`] belongs to.
#[derive(
    Component, Reflect, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
#[reflect(Component)]
#[relationship(relationship_target = Timeline)]
pub struct TargetTimeline(Entity);

/// The playback state of the [`Timeline`].
#[derive(Component, Default, Debug, Clone, Copy)]
pub enum TimelinePlayback {
    /// Playing in the forward direction with a time scale.
    Forward,
    /// Playing in the backward direction with a time scale.
    Backward,
    /// Not playing at the moment.
    #[default]
    Pause,
}

impl TimelinePlayback {
    #[inline]
    pub fn forward(&mut self) {
        *self = TimelinePlayback::Forward;
    }

    #[inline]
    pub fn backward(&mut self) {
        *self = TimelinePlayback::Backward;
    }

    #[inline]
    pub fn pause(&mut self) {
        *self = TimelinePlayback::Pause;
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub enum SequencePoint {
    /// The start of the [`Sequence`], normally at `0.0`.
    #[default]
    Start,
    /// The end of the [`Sequence`], normally at [`Sequence::duration()`].
    End,
    /// An exact time in the [`Sequence`].
    Exact(f32),
}

/// Determines the speed of the [`Timeline`] playback.
/// Consists of a correct-by-construction positive `f32` value .
#[derive(Component, Debug, Deref, Clone, Copy)]
pub struct TimeScale(f32);

impl Default for TimeScale {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl TimeScale {
    /// The provided value will be passed through
    /// [`f32::abs`] to ensure it is positive.
    pub const fn new(time_scale: f32) -> Self {
        Self(time_scale.abs())
    }

    /// The provided value will be passed through
    /// [`f32::abs`] to ensure it is positive.
    pub fn set(&mut self, time_scale: f32) {
        self.0 = time_scale.abs();
    }

    /// Consumes itself and returns the inner `f32` value.
    #[inline(always)]
    pub fn get(self) -> f32 {
        self.0
    }
}
