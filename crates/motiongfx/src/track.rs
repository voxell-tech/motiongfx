use core::time::Duration;

use alloc::boxed::Box;
use alloc::vec::Vec;
use field_path::field::UntypedField;
use hashbrown::HashMap;

use crate::action::{ActionClip, ActionKey};
use crate::sequence::Sequence;
use crate::time::IntoDuration;

pub trait TrackOrdering {
    /// Run all [`TrackFragment`]s one after another.
    fn ord_chain(self) -> TrackFragment;
    fn ord_all(self) -> TrackFragment;
    fn ord_any(self) -> TrackFragment;
    fn ord_flow(self, delay: impl IntoDuration) -> TrackFragment;
}

impl<T> TrackOrdering for T
where
    T: IntoIterator<Item = TrackFragment>,
{
    fn ord_chain(self) -> TrackFragment {
        chain(self)
    }

    fn ord_all(self) -> TrackFragment {
        all(self)
    }

    fn ord_any(self) -> TrackFragment {
        any(self)
    }

    fn ord_flow(self, delay: impl IntoDuration) -> TrackFragment {
        flow(delay, self)
    }
}

/// Run all [`TrackFragment`]s one after another.
#[must_use = "This function consumes all the given tracks and returns a modified one."]
pub fn chain(
    tracks: impl IntoIterator<Item = TrackFragment>,
) -> TrackFragment {
    let mut tracks_iter = tracks.into_iter();
    let mut track = tracks_iter.next().unwrap_or_default();

    let mut chain_duration = track.duration;

    for mut other_track in tracks_iter {
        for (key, mut other_sequence) in other_track.sequences.drain()
        {
            other_sequence.delay(chain_duration);
            track = track.upsert_sequence(key, other_sequence);
        }

        chain_duration =
            chain_duration.saturating_add(other_track.duration);
    }

    track.duration = chain_duration;
    track
}

/// Run all [`Track`]s concurrently and wait for all of them to finish.
#[must_use = "This function consumes all the given tracks and returns a modified one."]
pub fn all(
    tracks: impl IntoIterator<Item = TrackFragment>,
) -> TrackFragment {
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
pub fn any(
    tracks: impl IntoIterator<Item = TrackFragment>,
) -> TrackFragment {
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
    delay: impl IntoDuration,
    tracks: impl IntoIterator<Item = TrackFragment>,
) -> TrackFragment {
    let delay = delay.into_duration();
    let mut tracks_iter = tracks.into_iter();
    let mut track = tracks_iter.next().unwrap_or_default();

    let mut flow_delay = Duration::ZERO;
    let mut final_duration = track.duration;

    for other_track in tracks_iter {
        flow_delay = flow_delay.saturating_add(delay);
        final_duration = flow_delay
            .saturating_add(other_track.duration)
            .max(final_duration);

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
pub fn delay(
    delay: impl IntoDuration,
    mut track: TrackFragment,
) -> TrackFragment {
    let delay = delay.into_duration();

    for sequence in track.sequences.values_mut() {
        sequence.delay(delay);
    }

    track
}

pub struct TrackFragment {
    sequences: HashMap<ActionKey, Sequence>,
    duration: Duration,
}

impl TrackFragment {
    pub fn new() -> Self {
        Self {
            sequences: HashMap::new(),
            duration: Duration::ZERO,
        }
    }

    pub fn single(key: ActionKey, clip: ActionClip) -> Self {
        Self {
            duration: clip.duration,
            sequences: [(key, Sequence::new(clip))].into(),
        }
    }

    /// Updates or inserts a [`Sequence`] in a track.
    ///
    /// If the [`ActionKey`] already exists, this method appends the
    /// clips of the `new_sequence` to the existing sequence.
    /// If the [`ActionKey`] does not exist, a new entry is created
    /// for the `new_sequence`.
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
        key: ActionKey,
        new_sequence: Sequence,
    ) -> Self {
        match self.sequences.get_mut(&key) {
            Some(sequence) => {
                sequence.extend(new_sequence);
            }
            None => {
                self.sequences.insert(key, new_sequence);
            }
        }

        self
    }

    pub fn compile(self) -> Track {
        let mut sequences =
            self.sequences.into_iter().collect::<Vec<_>>();

        if sequences.is_empty() {
            return Track {
                field_lookups: Box::new([]),
                sequence_spans: Box::new([]),
                clip_arena: Box::new([]),
                duration: self.duration,
            };
        }

        sequences.sort_by_key(|(key, _)| *key.field());

        // The combinators accumulate `duration` independently of the
        // clip offsets, so pin them together here: the clamp in
        // `Timeline::set_target_time` must be able to reach the last
        // clip's end.
        let duration = sequences
            .iter()
            .map(|(_, seq)| seq.end())
            .max()
            .unwrap_or(Duration::ZERO)
            .max(self.duration);

        let mut seq_offset = 0;
        let mut sequence_spans = Vec::with_capacity(sequences.len());

        let mut field = sequences[0].0.field();
        let mut field_offset = 0;
        let mut field_len = 0;
        let mut field_lookups = Vec::new();

        for (key, seq) in sequences.iter() {
            sequence_spans.push((
                *key,
                Span {
                    offset: seq_offset,
                    len: seq.len(),
                },
            ));
            seq_offset += seq.len();

            if key.field() != field {
                field_lookups.push((
                    *field,
                    Span {
                        offset: field_offset,
                        len: field_len,
                    },
                ));

                field = key.field();
                field_offset = field_len;
                field_len = 0;
            }
            field_len += 1;
        }

        // Final field.
        field_lookups.push((
            *field,
            Span {
                offset: field_offset,
                len: field_len,
            },
        ));

        let clip_arena = sequences
            .into_iter()
            .flat_map(|(_, clips)| clips)
            .collect();

        Track {
            field_lookups: field_lookups.into_boxed_slice(),
            sequence_spans: sequence_spans.into_boxed_slice(),
            clip_arena,
            duration,
        }
    }
}

impl Default for TrackFragment {
    fn default() -> Self {
        Self::new()
    }
}

/// A compiled dense action sequences, optimized for playback and
/// queries.
///
/// A `Track` is created from a [`TrackFragment`] and provides an
/// immutable, space-efficient layout. [`ActionClip`]s are stored
/// in a flat array with spans for quick access.
#[derive(Debug, Clone)]
pub struct Track {
    // TODO: Use this to optimized baking/sampling? (There are no
    // use case for the lookups atm!)
    /// Lookup from each field to the range of actions affecting it.
    ///
    /// Each entry holds an [`UntypedField`] and a [`Span`] into
    /// `clip_spans`.
    field_lookups: Box<[(UntypedField, Span)]>,

    /// [`ActionClip`]s grouped by [`ActionKey`] in sorted order.
    ///
    /// Each entry holds an [`ActionKey`] and a [`Span`] into
    /// `clip_arena`.
    sequence_spans: Box<[(ActionKey, Span)]>,

    /// Contiguous storage of all action clips.
    clip_arena: Box<[ActionClip]>,

    /// Total duration of the track.
    ///
    /// Guaranteed to be `>=` the end of every clip in `clip_arena`.
    /// See [`TrackFragment::compile`].
    duration: Duration,
}

impl Track {
    pub fn lookup_field_spans(
        &self,
        field: impl Into<UntypedField>,
    ) -> Option<&[(ActionKey, Span)]> {
        let index = self
            .field_lookups
            .binary_search_by_key(&field.into(), |(f, _)| *f)
            .ok()?;

        let (_, span) = &self.field_lookups[index];

        Some(
            &self.sequence_spans[span.offset..span.offset + span.len],
        )
    }

    #[inline]
    pub fn field_lookups(&self) -> &[(UntypedField, Span)] {
        &self.field_lookups
    }

    #[inline]
    pub fn sequences_spans(&self) -> &[(ActionKey, Span)] {
        &self.sequence_spans
    }

    #[inline]
    pub fn clips(&self, span: Span) -> &[ActionClip] {
        &self.clip_arena[span.offset..span.offset + span.len]
    }

    #[inline]
    pub fn duration(&self) -> Duration {
        self.duration
    }
}

impl IntoIterator for Track {
    type Item = Self;

    type IntoIter = core::array::IntoIter<Self::Item, 1>;

    fn into_iter(self) -> Self::IntoIter {
        [self].into_iter()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub offset: usize,
    pub len: usize,
}

#[cfg(test)]
mod tests {
    use crate::action::{ActionId, IdRegistry, UntypedSubjectId};
    use crate::time::{ms, s};

    use super::*;

    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
    )]
    struct DummyId(u32);

    fn key(path: &'static str) -> ActionKey {
        ActionKey::new(
            UntypedSubjectId::PLACEHOLDER,
            UntypedField::placeholder_with_path(path),
        )
    }

    const fn clip(millis: u64) -> ActionClip {
        ActionClip::new(ActionId::PLACEHOLDER, ms(millis))
    }

    #[test]
    fn track_key_uniqueness() {
        // Sequence with 0 duration to prevent overlaps.
        const DUMMY_SEQ: Sequence = Sequence::new(clip(0));

        let entity1 = DummyId(1);
        let entity2 = DummyId(2);
        let field_u32_a = UntypedField::placeholder_with_path("a");
        let field_u32_b = UntypedField::placeholder_with_path("b");

        let mut id_registry = IdRegistry::new();
        let id1 = id_registry.register_instance(entity1);
        let id2 = id_registry.register_instance(entity2);

        let k1 = ActionKey::new(
            UntypedSubjectId::new::<DummyId>(id1),
            field_u32_a,
        );
        let k2 = ActionKey::new(
            UntypedSubjectId::new::<DummyId>(id2),
            field_u32_a,
        );
        let k3 = ActionKey::new(
            UntypedSubjectId::new::<DummyId>(id1),
            field_u32_b,
        );

        let track = TrackFragment::new()
            .upsert_sequence(k1, DUMMY_SEQ.clone())
            .upsert_sequence(k2, DUMMY_SEQ.clone())
            .upsert_sequence(k3, DUMMY_SEQ.clone())
            // Similar key with the first sequence.
            .upsert_sequence(k1, DUMMY_SEQ.clone());

        assert_eq!(track.sequences.len(), 3);
    }

    #[test]
    fn chain_duration_and_delay() {
        let track1 = TrackFragment::single(key("a"), clip(1000));
        let track2 = TrackFragment::single(key("b"), clip(2000));

        let track = [track1, track2].ord_chain();

        assert_eq!(track.duration, ms(3000));
        let seq_b = &track.sequences[&key("b")];
        // `seq_b` should be delayed by 1.0 (duration of `track1`).
        assert_eq!(seq_b.start(), ms(1000));
    }

    #[test]
    fn all_duration_max() {
        let track1 = TrackFragment::single(key("a"), clip(1000));
        let track2 = TrackFragment::single(key("b"), clip(3000));

        let track = [track1, track2].ord_all();
        assert_eq!(track.duration, ms(3000));
    }

    #[test]
    fn any_duration_min() {
        let track1 = TrackFragment::single(key("a"), clip(1000));
        let track2 = TrackFragment::single(key("b"), clip(3000));

        let track = [track1, track2].ord_any();
        assert_eq!(track.duration, ms(1000));
    }

    #[test]
    fn flow_with_delay() {
        let track1 = TrackFragment::single(key("a"), clip(1000));
        let track2 = TrackFragment::single(key("b"), clip(1000));

        let track = [track1, track2].ord_flow(ms(500));

        // 0.5 delay + 1.0 duration
        assert_eq!(track.duration, ms(1500));
        let seq_b = &track.sequences[&key("b")];
        // `seq_b` should be delayed by 0.5
        assert_eq!(seq_b.start(), ms(500));
    }

    #[test]
    fn delay_applies_offset() {
        let track = TrackFragment::single(key("a"), clip(2000));

        let track = delay(ms(1500), track);
        let seq_a = &track.sequences[&key("a")];

        assert_eq!(seq_a.start(), ms(1500));
        assert_eq!(seq_a.end(), ms(3500));
        assert_eq!(track.duration, ms(2000));
    }

    /// Chaining durations that have no exact `f32` representation used
    /// to leave `TrackFragment::duration` and the clip offsets on
    /// different values, because the two are accumulated separately.
    /// The mismatch tripped the non-overlap assert in `Sequence::push`
    /// and put `Track::duration` out of reach of the last clip's end.
    #[test]
    fn chain_accumulation_matches_clip_offsets() {
        // 0.1s is not representable in binary floating point.
        let tracks: Vec<_> = (0..10)
            .map(|_| TrackFragment::single(key("a"), clip(100)))
            .collect();

        let track = tracks.ord_chain();

        assert_eq!(track.duration, ms(1000));
        assert_eq!(track.sequences[&key("a")].end(), ms(1000));
    }

    /// `Track::duration` must always be reachable by the playhead, so
    /// that the final clip can resolve to `SampleMode::End`.
    #[test]
    fn compile_duration_covers_last_clip_end() {
        let mut fragment =
            TrackFragment::single(key("a"), clip(1000));
        // Understate the duration the way a combinator would if the
        // two accumulations ever diverged again.
        fragment.duration = ms(999);

        let track = fragment.compile();

        assert_eq!(track.duration(), ms(1000));
    }

    /// `IntoDuration` saturates absurd seconds to `Duration::MAX`, so
    /// the arithmetic downstream has to saturate too, or the panic just
    /// moves into `chain`, `flow`, or `ActionClip::end`.
    ///
    /// Distinct keys per fragment: saturated clips really do overlap,
    /// and the non-overlap assert is right to say so. Only the duration
    /// arithmetic is under test.
    #[test]
    fn saturated_durations_do_not_overflow_the_combinators() {
        let huge = |path: &'static str| {
            TrackFragment::single(
                key(path),
                ActionClip::new(
                    ActionId::PLACEHOLDER,
                    f32::INFINITY.into_duration(),
                ),
            )
        };

        assert_eq!(huge("a").duration, Duration::MAX);
        assert_eq!(
            [huge("a"), huge("b")].ord_chain().duration,
            Duration::MAX
        );
        assert_eq!(
            [huge("a"), huge("b")].ord_flow(s(1)).duration,
            Duration::MAX
        );
        assert_eq!(
            [huge("a"), huge("b")].ord_all().duration,
            Duration::MAX
        );
        assert_eq!(
            delay(f32::MAX, huge("a")).duration,
            Duration::MAX
        );
    }

    #[test]
    fn compile_empty_fragment_is_not_a_panic() {
        let track = TrackFragment::new().compile();

        assert_eq!(track.duration(), Duration::ZERO);
        assert!(track.sequences_spans().is_empty());
    }
}
