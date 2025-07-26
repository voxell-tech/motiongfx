use core::num::NonZeroUsize;

use bevy::prelude::*;
use nonempty::NonEmpty;
use smallvec::SmallVec;

use crate::sequence::{Sequence, SequenceController};
use crate::MotionGfxSet;

pub(super) struct TimelinePlugin;

impl Plugin for TimelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (apply_timeline_commands, update_target_time)
                .chain()
                .in_set(MotionGfxSet::TargetTime),
        );

        app.configure_sets(
            PostUpdate,
            (
                TimelineSet::Advance,
                TimelineSet::MarkAction,
                TimelineSet::Sample,
                TimelineSet::Sync,
            )
                .chain(),
        );

        app.add_systems(
            PostUpdate,
            (sync_target_index, sync_target_time)
                .in_set(TimelineSet::Sync),
        );
    }
}

pub trait CreateTimelineAppExt {
    fn create_timeline(
        &mut self,
        sequences: impl IntoIterator<Item = Sequence>,
    ) -> EntityCommands<'_>;
}

impl CreateTimelineAppExt for Commands<'_, '_> {
    /// Helper method to create timeline.
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

fn apply_timeline_commands(
    mut q_timelines: Query<&mut Timeline, Changed<Timeline>>,
    mut q_sequences: Query<(&Sequence, &mut SequenceController)>,
) -> Result {
    for mut timeline in q_timelines.iter_mut() {
        // Prevent infinite change to `Timeline`.
        let timeline = timeline.bypass_change_detection();

        let Some(command) = core::mem::take(&mut timeline.command)
        else {
            continue;
        };

        /// The index range affected by the sequence change.
        struct AffectedRange {
            /// The starting index.
            pub start: usize,
            /// The length after the starting index.
            pub len: NonZeroUsize,
            /// Determines if the affected range
            /// should move forward or backward.
            pub is_forward: bool,
        }

        struct GenericCommand {
            pub affected_range: Option<AffectedRange>,
            pub target_index: usize,
            pub sequence_point: SequencePoint,
        }

        let generic_command = match command {
            TimelineCommand::Next(sequence_point)
                if timeline.is_last_sequence() == false =>
            // No next sequence if we are already at the last one.
            {
                GenericCommand {
                    affected_range: Some(AffectedRange {
                        start: timeline.sequence_index(),
                        len: NonZeroUsize::MIN,
                        is_forward: true,
                    }),
                    target_index: timeline.sequence_index() + 1,
                    sequence_point,
                }
            }
            TimelineCommand::Previous(sequence_point)
                if timeline.is_first_sequence() == false =>
            // No previous sequence if we are already at the first one.
            {
                GenericCommand {
                    affected_range: Some(AffectedRange {
                        start: timeline.sequence_index(),
                        len: NonZeroUsize::MIN,
                        is_forward: false,
                    }),
                    target_index: timeline.sequence_index() - 1,
                    sequence_point,
                }
            }
            TimelineCommand::Current(sequence_point) => {
                GenericCommand {
                    affected_range: None,
                    target_index: timeline.sequence_index(),
                    sequence_point,
                }
            }
            TimelineCommand::Exact(index, sequence_point)
                if index < timeline.sequence_len() =>
            // Make sure the index is valid.
            {
                // No affected range if the target index is
                // equal to the current index.
                let affected_range = NonZeroUsize::new(
                    index.abs_diff(timeline.sequence_index()),
                )
                .map(|len| {
                    let is_forward =
                        index > timeline.sequence_index();
                    let mut start =
                        index.min(timeline.sequence_index);

                    if is_forward == false {
                        // Shift indices forward to prevent altering
                        // the target sequence.
                        start += 1;
                    }
                    AffectedRange {
                        start,
                        len,
                        is_forward,
                    }
                });

                GenericCommand {
                    affected_range,
                    target_index: timeline.sequence_index(),
                    sequence_point,
                }
            }
            _ => continue,
        };

        // Handle the affected range.
        if let Some(affected_range) = generic_command.affected_range {
            let set_target_time = if affected_range.is_forward {
                // Set to the end if moving forward.
                |sequence: &Sequence,
                 controller: &mut SequenceController| {
                    controller.target_time = sequence.duration();
                }
            } else {
                // Set to the start if moving backward.
                |_: &Sequence, controller: &mut SequenceController| {
                    controller.target_time = 0.0;
                }
            };

            for i in affected_range.start
                ..affected_range.start + affected_range.len.get()
            {
                let sequence_id = timeline.sequence_ids[i];
                let (sequence, mut controller) =
                    q_sequences.get_mut(sequence_id)?;

                // Set the target time based on the conditioned closure.
                set_target_time(sequence, &mut controller);
            }
        }

        // Apply command to the timeline.
        timeline.sequence_index = generic_command.target_index;

        let sequence_id = timeline
            .curr_sequence_id()
            .ok_or("No sequence in timeline!")?;

        // Apply the `SequencePoint` to the target sequence.
        let (sequence, mut controller) =
            q_sequences.get_mut(sequence_id)?;

        match generic_command.sequence_point {
            SequencePoint::Start => controller.target_time = 0.0,
            SequencePoint::End => {
                controller.target_time = sequence.duration()
            }
            SequencePoint::Exact(time) => {
                controller.target_time = time
            }
        }
    }

    Ok(())
}

/// Update [`SequenceController::target_time`] based on [`Timeline`].
fn update_target_time(
    q_timelines: Query<(&Timeline, &TimelinePlayback, &TimeScale)>,
    mut q_sequences: Query<(&Sequence, &mut SequenceController)>,
    time: Res<Time>,
) -> Result {
    for (timeline, playback, time_scale) in q_timelines.iter() {
        let Some(sequence_id) = timeline.curr_sequence_id() else {
            continue;
        };

        let (sequence, mut controller) =
            q_sequences.get_mut(sequence_id)?;

        let time_diff = time_scale.get() * time.delta_secs();
        match playback {
            TimelinePlayback::Forward
                if controller.curr_time() < sequence.duration() =>
            {
                controller.target_time += time_diff;
            }
            TimelinePlayback::Backward
                if controller.curr_time() > 0.0 =>
            {
                controller.target_time -= time_diff;
            }
            _ => continue,
        }
    }

    Ok(())
}

/// A command to control the [`Timeline`].
#[derive(Debug)]
pub enum TimelineCommand {
    /// Move to the next [`Sequence`] in the [`Timeline`]
    /// with a starting [`SequencePoint`].
    ///
    /// # Note
    ///
    /// This command has no effect if the current sequence is the last one.
    Next(SequencePoint),
    /// Move to the previous [`Sequence`] in the [`Timeline`]
    /// with a starting [`SequencePoint`].
    ///
    /// # Note
    ///
    /// This command has no effect if the current sequence is the first one.
    Previous(SequencePoint),
    /// Move to the [`SequencePoint`] in the current [`Sequence`]
    /// in the [`Timeline`].
    Current(SequencePoint),
    /// Move to an exact [`Sequence`] in the [`Timeline`]
    /// with a starting [`SequencePoint`].
    ///
    /// # Note
    ///
    /// This command has no effect if the target sequence does not exists.
    Exact(usize, SequencePoint),
}

fn sync_target_time(
    mut q_timeline_time: Query<
        &mut TimelineTime,
        Changed<TimelineTime>,
    >,
) {
    for mut timeline_time in q_timeline_time.iter_mut() {
        let timeline_time = timeline_time.bypass_change_detection();
        timeline_time.time = timeline_time.target_time;
    }
}

fn sync_target_index(
    mut q_timeline_index: Query<
        &mut TimelineIndex,
        Changed<TimelineIndex>,
    >,
) {
    for mut timeline_index in q_timeline_index.iter_mut() {
        let timeline_index = timeline_index.bypass_change_detection();
        timeline_index.index = timeline_index.target_index;
    }
}

/// The time controller for the [`Sequence`]s in the [`Timeline`].
///
/// The [`Sequence`] that will be controlled depends on [`TimelineIndex`].
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct TimelineTime {
    /// The current time of the current [`Sequence`] in the [`Timeline`].
    ///
    /// The current [`Sequence`] is based on [`TimelineIndex::index()`].
    time: f32,
    /// The target time of the target [`Sequence`] in the [`Timeline`].
    ///
    /// The target [`Sequence`] is based on [`TimelineIndex::target_index()`].
    target_time: f32,
}

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct TimelineIndex {
    /// The current sequence index in the [`Timeline`].
    index: u32,
    /// The target sequence index in the [`Timeline`].
    target_index: u32,
}

impl TimelineIndex {
    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn target_index(&self) -> u32 {
        self.target_index
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TimelineSet {
    Advance,
    MarkAction,
    Sample,
    Sync,
}

#[derive(Component, Debug, Clone)]
#[component(immutable)]
#[require(TimelineTime, TimelineIndex, TimelinePlayback, TimeScale)]
pub struct _Timeline {
    sequences: NonEmpty<Sequence>,
}

impl _Timeline {
    pub fn new() -> Self {
        Self {
            sequences: NonEmpty::new(Sequence::default()),
        }
    }
}

impl Default for _Timeline {
    fn default() -> Self {
        Self::new()
    }
}

impl _Timeline {
    pub fn chain(&mut self, sequence: Sequence) -> &mut Self {
        self.sequences.last_mut().chain(sequence);
        self
    }

    pub fn add_checkpoint(&mut self) -> &mut Self {
        self.sequences.push(Sequence::default());
        self
    }
}

/// Manipulates [`SequenceController::target_time`].
#[derive(Component, Debug)]
#[relationship_target(relationship = TargetTimeline, linked_spawn)]
#[require(TimelinePlayback, TimeScale)]
pub struct Timeline {
    /// The [`Sequence`]s that are related to this timeline.
    #[relationship]
    sequence_ids: SmallVec<[Entity; 1]>,
    /// The index in `sequence_ids`.
    sequence_index: usize,
    /// A deferred command that runs in the [`PostUpdate`] schedule.
    ///
    /// This will reset to [None] every frame after the command
    /// is being applied.
    command: Option<TimelineCommand>,
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

impl Timeline {
    /// Inserts a [`TimelineCommand`] that will run during [`PostUpdate`].
    ///
    /// This action will replace the previous the command if there's any.
    pub fn insert_command(&mut self, command: TimelineCommand) {
        self.command = Some(command);
    }
}

/// The target [`Timeline`] that this [`Sequence`] belongs to.
#[derive(
    Component, Reflect, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
#[reflect(Component)]
#[relationship(relationship_target = Timeline)]
pub struct TargetTimeline(Entity);

/// The point in time at the current [`Sequence`] of the [`Timeline`].
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum SequencePoint {
    /// The start of the [`Sequence`], normally at `0.0`.
    #[default]
    Start,
    /// The end of the [`Sequence`], normally at [`Sequence::duration()`].
    End,
    /// An exact time in the [`Sequence`].
    Exact(f32),
}

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

    /// Returns the inner `f32` value.
    #[inline(always)]
    #[must_use]
    pub fn get(&self) -> f32 {
        self.0
    }

    /// Consumes itself and returns the inner `f32` value.
    #[inline(always)]
    #[must_use]
    pub fn consume_get(self) -> f32 {
        self.0
    }
}
