//! # Timeline
//!
//! A `Timeline` is a series of tracks that run in a sequential order.
//!
//! When a timeline is _playing_, it will only advance 1 track at a
//! time and then pause, awaiting a trigger to proceed to a different
//! track. This design allows for manual control over the flow of
//! the timeline.

use bevy::prelude::*;
use nonempty::NonEmpty;

use crate::track::{Track, TrackBuilder};

pub struct TimelinePlugin;

impl Plugin for TimelinePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            (
                TimelineSet::Advance,
                TimelineSet::Mark,
                TimelineSet::Sample,
            )
                .chain(),
        );

        app.add_systems(
            PostUpdate,
            advance_timeline.before(TimelineSet::Advance),
        );
    }
}

/// Systems set for managing [`Timeline`] states.
/// Runs in the [`PostUpdate`] schedule.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TimelineSet {
    /// Advance the target time/index in [`Timeline`].
    Advance,
    /// Mark the segments that are going to be sampled this frame.
    Mark,
    /// Sample keyframes and applies the value.
    /// This happens before [`TransformSystem::TransformPropagate`].
    Sample,
}

/// Advance the [`Timeline`]'s current time if it's playing.
///
/// This system should run before the sampling starts.
fn advance_timeline(
    mut q_timelines: Query<&mut Timeline>,
    time: Res<Time>,
) {
    for mut timeline in q_timelines.iter_mut() {
        if !timeline.is_playing() {
            continue;
        }

        let increment = time.delta_secs() * timeline.time_scale;
        let target_time = timeline.target_time + increment;
        let duration = timeline.curr_track().duration();

        // Prevent time overshooting.
        timeline.target_time = target_time.clamp(0.0, duration);
    }
}

/// A compact series of track.
#[derive(Component, Debug)]
pub struct Timeline {
    tracks: Box<[Track]>,
    /// Determines if the timeline is currently playing.
    is_playing: bool,
    /// The time scale of the timeline. Set this to negative
    /// to play backwards.
    time_scale: f32,
    /// The current time of the current track.
    curr_time: f32,
    /// The target time of the target track.
    target_time: f32,
    /// The index of the current track.
    curr_index: usize,
    /// The index of the target track.
    target_index: usize,
}

// Getter methods.
impl Timeline {
    /// Returns whether the timeline is currently playing.
    #[inline]
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Returns the current time scaling factor.
    #[inline]
    pub fn time_scale(&self) -> f32 {
        self.time_scale
    }

    /// Returns the current playback time.
    #[inline]
    pub fn curr_time(&self) -> f32 {
        self.curr_time
    }

    /// Returns the target playback time.
    #[inline]
    pub fn target_time(&self) -> f32 {
        self.target_time
    }

    /// Returns the current track index.
    #[inline]
    pub fn curr_index(&self) -> usize {
        self.curr_index
    }

    /// Returns the target track index.
    #[inline]
    pub fn target_index(&self) -> usize {
        self.target_index
    }

    /// Returns a reference slice to all tracks in this timeline.
    #[inline]
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// Returns a reference to the current track.
    #[inline]
    pub fn curr_track(&self) -> &Track {
        &self.tracks[self.curr_index]
    }

    /// Returns a reference to the target track.
    #[inline]
    pub fn target_track(&self) -> &Track {
        &self.tracks[self.target_index]
    }

    /// Returns `true` if the current track is the last track.
    #[inline]
    pub fn is_last_track(&self) -> bool {
        self.curr_index == self.last_track_index()
    }

    /// Get the index of the last track. This is essentially the largest
    /// index you can provide in [`Timeline::set_target_track`].
    #[inline]
    pub fn last_track_index(&self) -> usize {
        self.tracks.len().saturating_sub(1)
    }
}

// Builder methods.
impl Timeline {
    #[inline]
    pub fn with_playing(mut self, play: bool) -> Self {
        self.is_playing = play;
        self
    }

    #[inline]
    pub fn with_time_scale(mut self, time_scale: f32) -> Self {
        self.time_scale = time_scale;
        self
    }

    #[inline]
    pub fn with_target_time(mut self, target_time: f32) -> Self {
        self.set_target_time(target_time);
        self
    }

    #[inline]
    pub fn with_target_track(mut self, target_index: usize) -> Self {
        self.set_target_track(target_index);
        self
    }
}

// Setter methods.
impl Timeline {
    pub fn set_playing(&mut self, play: bool) -> &mut Self {
        self.is_playing = play;
        self
    }

    pub fn set_time_scale(&mut self, time_scale: f32) -> &mut Self {
        self.time_scale = time_scale;
        self
    }

    /// Set the target time of the current track, clamping the value
    /// within \[0.0..=track.duration\]
    ///
    /// Warns if out of bounds in `debug_assertions`.
    pub fn set_target_time(&mut self, target_time: f32) -> &mut Self {
        let duration = self.target_track().duration();

        #[cfg(debug_assertions)]
        if target_time < 0.0 || target_time > duration {
            warn!(
                "Target time ({}) is out of bounds [0.0..={}].",
                target_time, duration
            );
        }

        self.target_time = target_time.clamp(0.0, duration);
        self
    }

    /// Set the target track index, clamping the value within
    /// \[0..=track_count - 1\].
    ///
    /// Warns if out of bounds in `debug_assertions`.
    pub fn set_target_track(
        &mut self,
        target_index: usize,
    ) -> &mut Self {
        let max_index = self.last_track_index();

        #[cfg(debug_assertions)]
        if target_index > max_index {
            warn!(
                "Target index ({}) is out of bounds [0..={}].",
                target_index, max_index
            );
        }

        self.target_index = target_index.clamp(0, max_index);
        self
    }

    pub(crate) fn sync_curr_time(&mut self) -> &mut Self {
        self.curr_time = self.target_time;
        self
    }

    pub(crate) fn sync_curr_track(&mut self) -> &mut Self {
        self.curr_index = self.target_index;
        self
    }
}

pub struct TimelineBuilder {
    tracks: NonEmpty<TrackBuilder>,
}

impl TimelineBuilder {
    pub fn new() -> Self {
        Self {
            tracks: NonEmpty::new(TrackBuilder::new()),
        }
    }
}

impl TimelineBuilder {
    /// Chain a track into the tail track in the timeline.
    pub fn chain(&mut self, track: TrackBuilder) -> &mut Self {
        let last_track = core::mem::take(self.tracks.last_mut());
        *self.tracks.last_mut() = last_track.chain(track);
        self
    }

    /// Creates the next track.
    pub fn add_checkpoint(&mut self) -> &mut Self {
        self.tracks.push(TrackBuilder::new());
        self
    }

    pub fn build(self) -> Timeline {
        Timeline {
            tracks: self
                .tracks
                .into_iter()
                .map(TrackBuilder::build)
                .collect(),
            is_playing: false,
            time_scale: 1.0,
            curr_time: 0.0,
            target_time: 0.0,
            curr_index: 0,
            target_index: 0,
        }
    }
}

impl Default for TimelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use bevy::ecs::system::RunSystemOnce;

    use crate::action::ActionSpan;
    use crate::sequence_v2::Sequence;
    use crate::track::{SequenceKey, TrackBuilder};

    use super::*;

    /// Creates a track with one dummy sequence with a given duration
    fn dummy_track(duration: f32) -> TrackBuilder {
        TrackBuilder::new_with_sequence(
            SequenceKey::placeholder(),
            Sequence::new(ActionSpan::new(
                Entity::PLACEHOLDER,
                duration,
            )),
        )
    }

    #[test]
    fn builder_chain_combines_tracks() {
        const T1: f32 = 1.0;
        const T2: f32 = 2.0;

        let track1 = dummy_track(T1);
        let track2 = dummy_track(T2);

        let mut builder = TimelineBuilder::new();
        builder.chain(track1).chain(track2);

        let timeline = builder.build();

        assert_eq!(timeline.tracks.len(), 1);
        assert_eq!(timeline.tracks[0].duration(), T1 + T2);
    }

    #[test]
    fn builder_checkpoint_creates_multiple_tracks() {
        const T1: f32 = 1.0;
        const T2: f32 = 2.0;

        let t1 = dummy_track(T1);
        let t2 = dummy_track(T2);

        let mut builder = TimelineBuilder::new();
        builder.chain(t1).add_checkpoint().chain(t2);

        let timeline = builder.build();

        assert_eq!(timeline.tracks.len(), 2);
        assert_eq!(timeline.tracks[0].duration(), T1);
        assert_eq!(timeline.tracks[1].duration(), T2);
    }

    // --- Systems: `advance_timeline` ---

    /// Create [`Time`] with a given delta seconds.
    fn time_with_delta(delta_secs: u64) -> Time {
        let mut time = Time::default();
        time.advance_by(Duration::from_secs(delta_secs));

        time
    }

    #[test]
    fn advance_timeline_increments_time_when_playing() {
        let mut world = World::new();

        // Insert Time resource with `delta = 1s`.
        world.insert_resource(time_with_delta(1));

        let mut builder = TimelineBuilder::new();
        builder.chain(dummy_track(5.0));

        let timeline = builder.build().with_playing(true);

        let entity = world.spawn(timeline).id();

        world.run_system_once(advance_timeline).unwrap();

        let timeline = world.get::<Timeline>(entity).unwrap();
        assert_eq!(timeline.target_time, 1.0);
    }

    #[test]
    fn advance_timeline_clamps_to_track_duration() {
        let mut world = World::new();

        // Insert Time resource with `delta = 2s`.
        world.insert_resource(time_with_delta(2));

        let mut builder = TimelineBuilder::new();
        builder.chain(dummy_track(1.5));

        let timeline = builder.build().with_playing(true);

        let entity = world.spawn(timeline).id();

        world.run_system_once(advance_timeline).unwrap();

        let timeline = world.get::<Timeline>(entity).unwrap();
        assert_eq!(timeline.target_time, 1.5);
    }

    #[test]
    fn advance_timeline_respects_time_scale() {
        let mut world = World::new();

        // Insert Time resource with `delta = 2s`.
        world.insert_resource(time_with_delta(2));

        let mut builder = TimelineBuilder::new();
        builder.chain(dummy_track(5.0));

        let timeline = builder
            .build()
            .with_playing(true)
            // Double speed.
            .with_time_scale(2.0);

        let entity = world.spawn(timeline).id();

        world.run_system_once(advance_timeline).unwrap();

        let timeline = world.get::<Timeline>(entity).unwrap();
        assert_eq!(timeline.target_time, 4.0);
    }
}
