use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use nonempty::NonEmpty;

use crate::action::ActionSpan;
use crate::field::FieldHash;

/// Run all [`Track`]s concurrently and wait for all of them to finish.
#[must_use]
pub fn all(tracks: impl IntoIterator<Item = Track>) -> Track {
    let mut tracks_iter = tracks.into_iter();
    let mut track = tracks_iter.next().unwrap_or_default();

    let mut max_duration = track.duration;

    for mut other_track in tracks_iter {
        max_duration = max_duration.max(other_track.duration);

        for (key, other_sequence) in other_track.sequences.drain() {
            track.insert_or_extend_sequence(key, other_sequence);
        }
    }

    track.duration = max_duration;
    track
}

/// Run all [`Track`]s concurrently and wait for any of them to finish.
#[must_use]
pub fn any(tracks: impl IntoIterator<Item = Track>) -> Track {
    let mut tracks_iter = tracks.into_iter();
    let mut track = tracks_iter.next().unwrap_or_default();

    let mut min_duration = track.duration;

    for mut other_track in tracks_iter {
        min_duration = min_duration.min(other_track.duration);

        for (key, other_sequence) in other_track.sequences.drain() {
            track.insert_or_extend_sequence(key, other_sequence);
        }
    }

    track.duration = min_duration;
    track
}

/// Run one [`Track`] after another with a fixed delay time.
#[must_use]
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
            track.insert_or_extend_sequence(key, sequence);
        }
    }

    track.duration = final_duration;
    track
}

/// Run a [`Track`] after a fixed delay time.
#[must_use]
pub fn delay(delay: f32, mut track: Track) -> Track {
    for sequence in track.sequences.values_mut() {
        sequence.delay(delay);
    }

    track
}

pub struct Timeline {
    tracks: NonEmpty<Track>,
}

impl Timeline {
    // // TODO: Refer implementation from sequence ordering!
    // pub fn chain(
    //     &mut self,
    //     key: TrackKey,
    //     span: ActionSpan,
    // ) -> &mut Self {
    //     let track = self.tracks.last_mut();

    //     match track.get_mut(&key) {
    //         Some(spans) => {
    //             spans.push_span(span);
    //         }
    //         None => {
    //             track.insert(key, SpanTrack::new(span));
    //         }
    //     }

    //     self
    // }
}

impl Timeline {
    pub fn new() -> Self {
        Self {
            tracks: NonEmpty::new(Track::new()),
        }
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Stores all uniquely identified tracks in the [`Sequence`],
/// mapped by a unique [`TrackKey`].
#[derive(Component, Debug, Clone)]
#[component(immutable)]
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
}

impl Track {
    pub fn insert_or_extend_sequence(
        &mut self,
        key: TrackKey,
        new_sequence: Sequence,
    ) {
        match self.sequences.get_mut(&key) {
            Some(sequence) => {
                sequence.extend(new_sequence.spans);
            }
            None => {
                self.sequences.insert(key, new_sequence);
            }
        }
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
    fn new(span: ActionSpan) -> Self {
        Self {
            spans: NonEmpty::new(span),
        }
    }

    #[inline]
    /// Get the start time of the span track.
    pub fn start_time(&self) -> f32 {
        self.spans.first().start_time()
    }

    #[inline]
    /// Get the end time of the span track.
    pub fn end_time(&self) -> f32 {
        self.spans.last().end_time()
    }

    /// Get the duration of the track.
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
            "`ActionSpan`s inside a `SpanTrack` shouldn't overlap!"
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

/// Stores the keys required to uniquely identify a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrackKey {
    /// The target entity that will be animated.
    action_target: Entity,
    /// The target field of the entity that will be animated.
    field_hash: FieldHash,
}

impl TrackKey {
    /// Get the target entity that will be animated.
    pub fn action_target(&self) -> Entity {
        self.action_target
    }

    /// Get the target field of the entity that will be animated.
    pub fn field_hash(&self) -> &FieldHash {
        &self.field_hash
    }
}
