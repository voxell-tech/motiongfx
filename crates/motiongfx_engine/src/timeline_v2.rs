//! # Timeline
//!
//! A `Timeline` is a series of tracks that run in a sequential order.
//!
//! When a timeline is _playing_, it will only advance 1 track at a
//! time and then pause, awaiting a trigger to proceed to a different
//! track. This design allows for manual control over the flow of
//! the timeline.

use bevy::ecs::query::QueryData;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use nonempty::NonEmpty;

use crate::action::ActionSpan;
use crate::field::FieldHash;

pub struct TimelinePlugin;

impl Plugin for TimelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                advance_timeline.before(TimelineSet::Advance),
                sync_timeline.after(TimelineSet::Sync),
            ),
        );
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TimelineSet {
    /// Advance the target time/index in [`Timeline`].
    Advance,
    /// Sample keyframes and applies the value.
    /// This happens before [`TransformSystem::TransformPropagate`].
    Sample,
    /// Sync the current time/index with the target in [`Timeline`].
    Sync,
}

/// Advance the [`Timeline`]'s current time if it's playing.
///
/// This system should run before the sampling starts.
fn advance_timeline(
    mut q_states: Query<(&Timeline, &mut TimelineState)>,
    time: Res<Time>,
) {
    for (timeline, mut state) in q_states.iter_mut() {
        if !state.is_playing {
            continue;
        }

        let increment = time.delta_secs() * state.time_scale;
        let target_time = state.target_time + increment;
        let duration = timeline.tracks[state.curr_index].duration;

        // Prevent time overshooting.
        state.target_time = target_time.clamp(0.0, duration);
    }
}

/// Sync [`Timeline`]'s current time and index with the target.
///
/// This system should run after the sampling is completed.
fn sync_timeline(
    mut q_states: Query<&mut TimelineState, Changed<TimelineState>>,
) {
    for mut state in q_states.iter_mut() {
        let state = state.bypass_change_detection();
        state.curr_time = state.target_time;
        state.curr_index = state.target_index;
    }
}

#[derive(Component, Debug)]
#[component(immutable)]
#[require(TimelineState)]
pub struct Timeline {
    tracks: Box<[Track]>,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct TimelineState {
    is_playing: bool,
    time_scale: f32,
    curr_time: f32,
    target_time: f32,
    curr_index: usize,
    target_index: usize,
}

impl TimelineState {
    pub const fn new() -> Self {
        Self {
            is_playing: false,
            time_scale: 1.0,
            curr_time: 0.0,
            target_time: 0.0,
            curr_index: 0,
            target_index: 0,
        }
    }
}

impl TimelineState {
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn time_scale(&self) -> f32 {
        self.time_scale
    }

    pub fn curr_time(&self) -> f32 {
        self.curr_time
    }

    pub fn target_time(&self) -> f32 {
        self.target_time
    }

    pub fn curr_index(&self) -> usize {
        self.curr_index
    }

    pub fn target_index(&self) -> usize {
        self.target_index
    }
}

impl Default for TimelineState {
    fn default() -> Self {
        Self::new()
    }
}

impl TimelineState {
    pub fn set_play(&mut self, play: bool) -> &mut Self {
        self.is_playing = play;
        self
    }

    pub fn set_time_scale(&mut self, time_scale: f32) -> &mut Self {
        self.time_scale = time_scale;
        self
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct TimelineCtx<'w> {
    pub timeline: &'w Timeline,
    pub state: Mut<'w, TimelineState>,
}

impl<'w> TimelineCtx<'w> {
    pub fn current_track(&self) -> &Track {
        &self.timeline.tracks[self.state.curr_index]
    }

    /// Set the target time of the current track, clamping the value
    /// within \[0.0..=track.duration\]
    pub fn set_target_time(&mut self, target_time: f32) -> &mut Self {
        let duration = self.current_track().duration;

        #[cfg(debug_assertions)]
        if target_time < 0.0 || target_time > duration {
            warn!("Target time ({target_time}) is out of bound [0.0..={duration}].");
        }

        self.state.target_time = target_time.clamp(0.0, duration);
        self
    }

    /// Set the target track index, clamping the value within
    /// \[0..=track_count - 1\].
    pub fn set_target_index(
        &mut self,
        target_index: usize,
    ) -> &mut Self {
        let max_index = self.timeline.tracks.len().saturating_sub(1);

        #[cfg(debug_assertions)]
        if target_index > max_index {
            warn!("Target index ({target_index}) is out of bound [0..={max_index}].");
        }

        self.state.target_index = target_index.clamp(0, max_index);
        self
    }
}

fn test(mut q_test: Query<(&Name, TimelineCtx)>) {
    for (name, mut query) in q_test.iter_mut() {
        query.state.bypass_change_detection().is_playing = false;
    }
}

pub struct TimelineBuilder {
    tracks: NonEmpty<Track>,
}

impl TimelineBuilder {
    pub fn new() -> Self {
        Self {
            tracks: NonEmpty::new(Track::new()),
        }
    }
}

impl TimelineBuilder {
    /// Chain a track into the tail track in the timeline.
    pub fn chain(&mut self, track: Track) -> &mut Self {
        let last_track = core::mem::take(self.tracks.last_mut());
        *self.tracks.last_mut() = last_track.chain(track);
        self
    }

    /// Creates the next track.
    pub fn add_checkpoint(&mut self) -> &mut Self {
        self.tracks.push(Track::new());
        self
    }

    pub fn build(self) -> Timeline {
        Timeline {
            tracks: self.tracks.into_iter().collect(),
        }
    }
}

impl Default for TimelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Responsible for storing all identified
/// [`Sequence`]s, mapped by a unique [`TrackKey`].
#[derive(Debug, Clone)]
pub struct Track {
    sequences: HashMap<TrackKey, Sequence>,
    duration: f32,
}

impl Track {
    pub fn new() -> Self {
        Self {
            sequences: HashMap::new(),
            duration: 0.0,
        }
    }

    pub fn new_with_sequence(
        key: TrackKey,
        sequence: Sequence,
    ) -> Self {
        Self {
            duration: sequence.duration(),
            sequences: [(key, sequence)].into(),
        }
    }
}

impl Track {
    #[inline]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Updates or inserts a [`Sequence`] in a track.
    ///
    /// If the [`TrackKey`] already exists, this method appends the spans
    /// of the `new_sequence` to the existing sequence. If the [`TrackKey`]
    /// does not exist, a new entry is created for the `new_sequence`.
    ///
    /// This method consumes `self` and returns a modified instance,
    /// following a builder pattern.
    ///
    /// # Parameters
    ///
    /// * `key`: The unique identifier for the track.
    /// * `new_sequence`: The sequence to be added or extended.
    pub fn upsert_sequence(
        mut self,
        key: TrackKey,
        new_sequence: Sequence,
    ) -> Self {
        match self.sequences.get_mut(&key) {
            Some(sequence) => {
                sequence.extend(new_sequence.spans);
            }
            None => {
                self.sequences.insert(key, new_sequence);
            }
        }

        self
    }

    #[inline]
    pub fn delay(mut self, duration: f32) -> Self {
        delay(duration, self)
    }

    #[inline]
    pub fn chain(self, other: Self) -> Self {
        chain([self, other])
    }
}

impl Default for Track {
    fn default() -> Self {
        Self::new()
    }
}

/// A non-overlapping sequence of [`ActionSpan`]s.
#[derive(Debug, Clone)]
pub struct Sequence {
    spans: NonEmpty<ActionSpan>,
}

impl Sequence {
    pub const fn new(span: ActionSpan) -> Self {
        Self {
            spans: NonEmpty::new(span),
        }
    }

    /// Get the start time of the span track.
    #[inline]
    #[must_use]
    pub fn start_time(&self) -> f32 {
        self.spans.first().start_time()
    }

    /// Get the end time of the span track.
    #[inline]
    #[must_use]
    pub fn end_time(&self) -> f32 {
        self.spans.last().end_time()
    }

    /// Get the duration of the track.
    #[inline]
    #[must_use]
    pub fn duration(&self) -> f32 {
        self.end_time() - self.start_time()
    }
}

impl Sequence {
    pub fn delay(&mut self, delay: f32) {
        for span in self.spans.iter_mut() {
            span.delay(delay);
        }
    }

    #[inline]
    pub fn push(&mut self, span: ActionSpan) {
        debug_assert!(
            span.start_time() >= self.end_time(),
            "({} >= {}) `ActionSpan`s inside a `Sequence` shouldn't overlap!",
            span.start_time(),
            self.end_time(),
        );

        self.spans.push(span);
    }
}

impl Extend<ActionSpan> for Sequence {
    fn extend<T: IntoIterator<Item = ActionSpan>>(
        &mut self,
        iter: T,
    ) {
        #[cfg(debug_assertions)]
        {
            for span in iter.into_iter() {
                self.push(span);
            }
        }
        #[cfg(not(debug_assertions))]
        self.spans.extend(iter);
    }
}

/// Key that uniquely identifies a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrackKey {
    /// The target entity that will be animated.
    target: Entity,
    /// The target field of the entity that will be animated.
    field_hash: FieldHash,
}

impl TrackKey {
    /// Get the target entity that will be animated.
    pub fn target(&self) -> Entity {
        self.target
    }

    /// Get the target field of the entity that will be animated.
    pub fn field_hash(&self) -> &FieldHash {
        &self.field_hash
    }
}

mod track {}

pub trait TrackOrdering {
    fn chain(self) -> Track;
    fn all(self) -> Track;
    fn any(self) -> Track;
    fn flow(self, delay: f32) -> Track;
}

impl<T> TrackOrdering for T
where
    T: IntoIterator<Item = Track>,
{
    fn chain(self) -> Track {
        chain(self)
    }

    fn all(self) -> Track {
        all(self)
    }

    fn any(self) -> Track {
        any(self)
    }

    fn flow(self, delay: f32) -> Track {
        flow(delay, self)
    }
}

/// Run all [`Track`]s concurrently and wait for all of them to finish.
#[must_use = "This function consumes all the given tracks and returns a modified one."]
pub fn chain(tracks: impl IntoIterator<Item = Track>) -> Track {
    let mut tracks_iter = tracks.into_iter();
    let mut track = tracks_iter.next().unwrap_or_default();

    let mut chain_duration = track.duration;

    for mut other_track in tracks_iter {
        for (key, mut other_sequence) in other_track.sequences.drain()
        {
            other_sequence.delay(chain_duration);
            track = track.upsert_sequence(key, other_sequence);
        }

        chain_duration += other_track.duration;
    }

    track.duration = chain_duration;
    track
}

/// Run all [`Track`]s concurrently and wait for all of them to finish.
#[must_use = "This function consumes all the given tracks and returns a modified one."]
pub fn all(tracks: impl IntoIterator<Item = Track>) -> Track {
    let mut tracks_iter = tracks.into_iter();
    let mut track = tracks_iter.next().unwrap_or_default();

    let mut max_duration = track.duration;

    for mut other_track in tracks_iter {
        max_duration = max_duration.max(other_track.duration);

        for (key, other_sequence) in other_track.sequences.drain() {
            track = track.upsert_sequence(key, other_sequence);
        }
    }

    track.duration = max_duration;
    track
}

/// Run all [`Track`]s concurrently and wait for any of them to finish.
#[must_use = "This function consumes all the given tracks and returns a modified one."]
pub fn any(tracks: impl IntoIterator<Item = Track>) -> Track {
    let mut tracks_iter = tracks.into_iter();
    let mut track = tracks_iter.next().unwrap_or_default();

    let mut min_duration = track.duration;

    for mut other_track in tracks_iter {
        min_duration = min_duration.min(other_track.duration);

        for (key, other_sequence) in other_track.sequences.drain() {
            track = track.upsert_sequence(key, other_sequence);
        }
    }

    track.duration = min_duration;
    track
}

/// Run one [`Track`] after another with a fixed delay time.
#[must_use = "This function consumes all the given tracks and returns a modified one."]
pub fn flow(
    delay: f32,
    tracks: impl IntoIterator<Item = Track>,
) -> Track {
    let mut tracks_iter = tracks.into_iter();
    let mut track = tracks_iter.next().unwrap_or_default();

    let mut flow_delay = 0.0;
    let mut final_duration = track.duration;

    for other_track in tracks_iter {
        flow_delay += delay;
        final_duration =
            (flow_delay + other_track.duration).max(final_duration);

        for (key, mut sequence) in other_track.sequences {
            sequence.delay(flow_delay);
            track = track.upsert_sequence(key, sequence);
        }
    }

    track.duration = final_duration;
    track
}

/// Run a [`Track`] after a fixed delay time.
#[must_use = "This function consumes the given track and returns a modified one."]
pub fn delay(delay: f32, mut track: Track) -> Track {
    for sequence in track.sequences.values_mut() {
        sequence.delay(delay);
    }

    track
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::entity::Entity;

    fn key(name: &'static str) -> TrackKey {
        TrackKey {
            target: Entity::PLACEHOLDER,
            field_hash: FieldHash::new::<u32>(name),
        }
    }

    const fn span(duration: f32) -> ActionSpan {
        ActionSpan::new(Entity::PLACEHOLDER, duration)
    }

    #[test]
    fn track_key_uniqueness() {
        const DUMMY_SEQ: Sequence = Sequence::new(span(1.0));

        let entity1 = Entity::from_raw(1);
        let entity2 = Entity::from_raw(2);
        let field_u32_a = FieldHash::new::<u32>("a");
        let field_u32_b = FieldHash::new::<u32>("b");

        let track1 = Track::new_with_sequence(
            TrackKey {
                target: entity1,
                field_hash: field_u32_a,
            },
            DUMMY_SEQ.clone(),
        );
        let track2 = Track::new_with_sequence(
            TrackKey {
                target: entity2,
                field_hash: field_u32_a,
            },
            DUMMY_SEQ.clone(),
        );
        let track3 = Track::new_with_sequence(
            TrackKey {
                target: entity1,
                field_hash: field_u32_b,
            },
            DUMMY_SEQ.clone(),
        );
        // Similar key with `track1`.
        let track4 = Track::new_with_sequence(
            TrackKey {
                target: entity1,
                field_hash: field_u32_a,
            },
            DUMMY_SEQ.clone(),
        );

        let track = [track1, track2, track3, track4].chain();
        assert_eq!(track.sequences.len(), 3);
    }

    #[test]
    fn chain_duration_and_delay() {
        let track0 = Track::new_with_sequence(
            key("a"),
            Sequence::new(span(1.0)),
        );
        let track1 = Track::new_with_sequence(
            key("b"),
            Sequence::new(span(2.0)),
        );

        let track = [track0, track1].chain();

        assert_eq!(track.duration, 3.0);
        let seq_b = &track.sequences[&key("b")];
        // `seq_b` should be delayed by 1.0 (duration of `track0`).
        assert_eq!(seq_b.start_time(), 1.0);
    }

    #[test]
    fn all_duration_max() {
        let track0 = Track::new_with_sequence(
            key("a"),
            Sequence::new(span(1.0)),
        );
        let track1 = Track::new_with_sequence(
            key("b"),
            Sequence::new(span(3.0)),
        );

        let track = [track0, track1].all();
        assert_eq!(track.duration, 3.0);
    }

    #[test]
    fn any_duration_min() {
        let track0 = Track::new_with_sequence(
            key("a"),
            Sequence::new(span(1.0)),
        );
        let track1 = Track::new_with_sequence(
            key("b"),
            Sequence::new(span(3.0)),
        );

        let track = [track0, track1].any();
        assert_eq!(track.duration, 1.0);
    }

    #[test]
    fn flow_with_delay() {
        let track0 = Track::new_with_sequence(
            key("a"),
            Sequence::new(span(1.0)),
        );
        let track1 = Track::new_with_sequence(
            key("b"),
            Sequence::new(span(1.0)),
        );

        let track = [track0, track1].flow(0.5);

        assert_eq!(track.duration, 1.5); // 0.5 delay + 1.0 duration
        let seq_b = &track.sequences[&key("b")];
        // `seq_b` should be delayed by 0.5
        assert_eq!(seq_b.start_time(), 0.5);
    }

    #[test]
    fn delay_applies_offset() {
        let track = Track::new_with_sequence(
            key("a"),
            Sequence::new(span(2.0)),
        );

        let track = delay(1.5, track);
        let seq_a = &track.sequences[&key("a")];

        assert_eq!(seq_a.start_time(), 1.5);
        assert_eq!(seq_a.end_time(), 3.5);
        assert_eq!(track.duration, 2.0);
    }
}

#[cfg(test)]
mod timeline_tests {
    use core::time::Duration;

    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// Creates a track with one dummy sequence with a given duration
    fn dummy_track(duration: f32) -> Track {
        Track::new_with_sequence(
            TrackKey {
                target: Entity::PLACEHOLDER,
                field_hash: FieldHash::new::<u32>("dummy"),
            },
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

        let t1 = dummy_track(T1);
        let t2 = dummy_track(T2);

        let mut builder = TimelineBuilder::new();
        builder.chain(t1).chain(t2);

        let timeline = builder.build();

        assert_eq!(timeline.tracks.len(), 1);
        assert_eq!(timeline.tracks[0].duration, T1 + T2);
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
        assert_eq!(timeline.tracks[0].duration, T1);
        assert_eq!(timeline.tracks[1].duration, T2);
    }

    // --- Systems: advance_timeline ---

    /// Create [`Time`] with a given delta seconds.
    fn time_with_delta(delta_secs: u64) -> Time<()> {
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

        let timeline = builder.build();

        let mut state = TimelineState::new();
        state.set_play(true);

        let entity = world.spawn((timeline, state)).id();

        world.run_system_once(advance_timeline).unwrap();

        let state = world.get::<TimelineState>(entity).unwrap();
        assert_eq!(state.target_time, 1.0);
    }

    #[test]
    fn advance_timeline_clamps_to_track_duration() {
        let mut world = World::new();

        // Insert Time resource with `delta = 2s`.
        world.insert_resource(time_with_delta(2));

        let mut builder = TimelineBuilder::new();
        builder.chain(dummy_track(1.5));

        let timeline = builder.build();

        let mut state = TimelineState::new();
        state.set_play(true);

        let entity = world.spawn((timeline, state)).id();

        world.run_system_once(advance_timeline).unwrap();

        let state = world.get::<TimelineState>(entity).unwrap();
        assert_eq!(state.target_time, 1.5);
    }

    #[test]
    fn advance_timeline_respects_time_scale() {
        let mut world = World::new();

        // Insert Time resource with `delta = 2s`.
        world.insert_resource(time_with_delta(2));

        let mut builder = TimelineBuilder::new();
        builder.chain(dummy_track(5.0));

        let timeline = builder.build();

        let mut state = TimelineState::new();
        state
            .set_play(true)
            // Double speed.
            .set_time_scale(2.0);

        let entity = world.spawn((timeline, state)).id();

        world.run_system_once(advance_timeline).unwrap();

        let state = world.get::<TimelineState>(entity).unwrap();
        assert_eq!(state.target_time, 4.0);
    }

    // --- Systems: sync_timeline ---

    #[test]
    fn sync_timeline_copies_target_to_current() {
        let mut world = World::new();

        let mut state = TimelineState::new();
        state.target_time = 1.5;
        state.target_index = 2;

        let entity = world.spawn(state).id();

        world.run_system_once(sync_timeline).unwrap();

        let state = world.get::<TimelineState>(entity).unwrap();
        assert_eq!(state.curr_time, 1.5);
        assert_eq!(state.curr_index, 2);
    }
}
