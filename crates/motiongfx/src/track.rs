use alloc::boxed::Box;
use alloc::vec::Vec;
use field_path::field::UntypedField;
use hashbrown::HashMap;

use crate::action::{ActionClip, ActionKey};
use crate::sequence::Sequence;

#[cfg(feature = "metadata")]
mod meta;
#[cfg(feature = "metadata")]
pub use meta::{Combinator, FragmentKind, FragmentMeta};

pub trait TrackOrdering {
    /// Run all [`TrackFragment`]s one after another.
    fn ord_chain(self) -> TrackFragment;
    fn ord_all(self) -> TrackFragment;
    fn ord_any(self) -> TrackFragment;
    fn ord_flow(self, delay: f32) -> TrackFragment;
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

    fn ord_flow(self, delay: f32) -> TrackFragment {
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
    #[cfg(feature = "metadata")]
    let mut children = alloc::vec![track.meta.clone()];

    for mut other_track in tracks_iter {
        for (key, mut other_sequence) in other_track.sequences.drain()
        {
            other_sequence.delay(chain_duration);
            track = track.upsert_sequence(key, other_sequence);
        }

        #[cfg(feature = "metadata")]
        {
            let mut meta = other_track.meta;
            meta.shift(chain_duration);
            children.push(meta);
        }

        chain_duration += other_track.duration;
    }

    track.duration = chain_duration;
    #[cfg(feature = "metadata")]
    {
        track.meta = FragmentMeta::group(
            Combinator::Chain,
            chain_duration,
            children,
        );
    }
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
    #[cfg(feature = "metadata")]
    let mut children = alloc::vec![track.meta.clone()];

    for mut other_track in tracks_iter {
        max_duration = max_duration.max(other_track.duration);

        for (key, other_sequence) in other_track.sequences.drain() {
            track = track.upsert_sequence(key, other_sequence);
        }

        // Concurrent: children keep their own starts (no shift).
        #[cfg(feature = "metadata")]
        children.push(other_track.meta);
    }

    track.duration = max_duration;
    #[cfg(feature = "metadata")]
    {
        track.meta = FragmentMeta::group(
            Combinator::All,
            max_duration,
            children,
        );
    }
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
    #[cfg(feature = "metadata")]
    let mut children = alloc::vec![track.meta.clone()];

    for mut other_track in tracks_iter {
        min_duration = min_duration.min(other_track.duration);

        for (key, other_sequence) in other_track.sequences.drain() {
            track = track.upsert_sequence(key, other_sequence);
        }

        // Concurrent: children keep their own starts (no shift).
        #[cfg(feature = "metadata")]
        children.push(other_track.meta);
    }

    track.duration = min_duration;
    #[cfg(feature = "metadata")]
    {
        track.meta = FragmentMeta::group(
            Combinator::Any,
            min_duration,
            children,
        );
    }
    track
}

/// Run one [`Track`] after another with a fixed delay time.
#[must_use = "This function consumes all the given tracks and returns a modified one."]
pub fn flow(
    delay: f32,
    tracks: impl IntoIterator<Item = TrackFragment>,
) -> TrackFragment {
    let mut tracks_iter = tracks.into_iter();
    let mut track = tracks_iter.next().unwrap_or_default();

    let mut flow_delay = 0.0;
    let mut final_duration = track.duration;
    #[cfg(feature = "metadata")]
    let mut children = alloc::vec![track.meta.clone()];

    for other_track in tracks_iter {
        flow_delay += delay;
        final_duration =
            (flow_delay + other_track.duration).max(final_duration);

        #[cfg(feature = "metadata")]
        {
            let mut meta = other_track.meta;
            meta.shift(flow_delay);
            children.push(meta);
        }

        for (key, mut sequence) in other_track.sequences {
            sequence.delay(flow_delay);
            track = track.upsert_sequence(key, sequence);
        }
    }

    track.duration = final_duration;
    #[cfg(feature = "metadata")]
    {
        track.meta = FragmentMeta::group(
            Combinator::Flow(delay),
            final_duration,
            children,
        );
    }
    track
}

/// Run a [`Track`] after a fixed delay time.
#[must_use = "This function consumes the given track and returns a modified one."]
pub fn delay(delay: f32, mut track: TrackFragment) -> TrackFragment {
    for sequence in track.sequences.values_mut() {
        sequence.delay(delay);
    }
    // A delay is just an offset, so shift the subtree without wrapping
    // it in a new group node.
    #[cfg(feature = "metadata")]
    track.meta.shift(delay);

    track
}

pub struct TrackFragment {
    sequences: HashMap<ActionKey, Sequence>,
    duration: f32,
    #[cfg(feature = "metadata")]
    meta: FragmentMeta,
}

impl TrackFragment {
    pub fn new() -> Self {
        Self {
            sequences: HashMap::new(),
            duration: 0.0,
            #[cfg(feature = "metadata")]
            meta: FragmentMeta::empty(),
        }
    }

    pub fn single(key: ActionKey, clip: ActionClip) -> Self {
        Self {
            duration: clip.duration,
            #[cfg(feature = "metadata")]
            meta: FragmentMeta::leaf(key, &clip),
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
        sequences.sort_by_key(|(key, _)| *key.field());

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
            duration: self.duration,
            #[cfg(feature = "metadata")]
            meta: self.meta,
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

    /// Total duration of the track in seconds.
    duration: f32,

    /// Composition tree describing how the source [`TrackFragment`]s
    /// were combined. See [`FragmentMeta`].
    #[cfg(feature = "metadata")]
    meta: FragmentMeta,
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

    /// The composition tree describing how this track's fragments were
    /// combined (chain / all / any / flow).
    #[cfg(feature = "metadata")]
    #[inline]
    pub fn meta(&self) -> &FragmentMeta {
        &self.meta
    }

    #[inline]
    pub fn clips(&self, span: Span) -> &[ActionClip] {
        &self.clip_arena[span.offset..span.offset + span.len]
    }

    #[inline]
    pub fn duration(&self) -> f32 {
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

    const fn clip(duration: f32) -> ActionClip {
        ActionClip::new(ActionId::PLACEHOLDER, duration)
    }

    #[test]
    fn track_key_uniqueness() {
        // Sequence with 0 duration to prevent overlaps.
        const DUMMY_SEQ: Sequence = Sequence::new(clip(0.0));

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
        let track1 = TrackFragment::single(key("a"), clip(1.0));
        let track2 = TrackFragment::single(key("b"), clip(2.0));

        let track = [track1, track2].ord_chain();

        assert_eq!(track.duration, 3.0);
        let seq_b = &track.sequences[&key("b")];
        // `seq_b` should be delayed by 1.0 (duration of `track1`).
        assert_eq!(seq_b.start(), 1.0);
    }

    #[test]
    fn all_duration_max() {
        let track1 = TrackFragment::single(key("a"), clip(1.0));
        let track2 = TrackFragment::single(key("b"), clip(3.0));

        let track = [track1, track2].ord_all();
        assert_eq!(track.duration, 3.0);
    }

    #[test]
    fn any_duration_min() {
        let track1 = TrackFragment::single(key("a"), clip(1.0));
        let track2 = TrackFragment::single(key("b"), clip(3.0));

        let track = [track1, track2].ord_any();
        assert_eq!(track.duration, 1.0);
    }

    #[test]
    fn flow_with_delay() {
        let track1 = TrackFragment::single(key("a"), clip(1.0));
        let track2 = TrackFragment::single(key("b"), clip(1.0));

        let track = [track1, track2].ord_flow(0.5);

        assert_eq!(track.duration, 1.5); // 0.5 delay + 1.0 duration
        let seq_b = &track.sequences[&key("b")];
        // `seq_b` should be delayed by 0.5
        assert_eq!(seq_b.start(), 0.5);
    }

    #[test]
    fn delay_applies_offset() {
        let track = TrackFragment::single(key("a"), clip(2.0));

        let track = delay(1.5, track);
        let seq_a = &track.sequences[&key("a")];

        assert_eq!(seq_a.start(), 1.5);
        assert_eq!(seq_a.end(), 3.5);
        assert_eq!(track.duration, 2.0);
    }

    #[cfg(feature = "metadata")]
    mod metadata {
        use super::*;

        /// `[all(a, b), c].chain()` should record the nesting and the
        /// absolute start of each node.
        #[test]
        fn nested_tree_starts() {
            let a = TrackFragment::single(key("a"), clip(1.0));
            let b = TrackFragment::single(key("b"), clip(2.0));
            let c = TrackFragment::single(key("c"), clip(1.0));

            let track = [[a, b].ord_all(), c].ord_chain();
            let root = &track.meta;

            // chain(all(a,b) @0 dur2, c @2 dur1) -> total 3.
            assert_eq!(root.start, 0.0);
            assert_eq!(root.duration, 3.0);
            let FragmentKind::Group {
                combinator: Combinator::Chain,
                children,
            } = &root.kind
            else {
                panic!("root should be a Chain group");
            };
            assert_eq!(children.len(), 2);

            // First child: all(a, b), concurrent from 0, dur = max = 2.
            let all = &children[0];
            assert_eq!((all.start, all.duration), (0.0, 2.0));
            let FragmentKind::Group {
                combinator: Combinator::All,
                children: inner,
            } = &all.kind
            else {
                panic!("first child should be an All group");
            };
            assert_eq!(inner.len(), 2);
            // Concurrent leaves both start at 0.
            assert!(inner.iter().all(|leaf| leaf.start == 0.0));

            // Second child: leaf c, chained after the all group (@2).
            let leaf_c = &children[1];
            assert_eq!((leaf_c.start, leaf_c.duration), (2.0, 1.0));
            assert!(matches!(leaf_c.kind, FragmentKind::Leaf { .. }));
        }

        /// `delay` shifts the subtree without adding a group node.
        #[test]
        fn delay_shifts_leaf() {
            let track = delay(
                1.5,
                TrackFragment::single(key("a"), clip(2.0)),
            );
            assert_eq!(track.meta.start, 1.5);
            assert!(matches!(
                track.meta.kind,
                FragmentKind::Leaf { .. }
            ));
        }
    }
}
