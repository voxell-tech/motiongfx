use core::marker::PhantomData;
use core::time::Duration;

use alloc::boxed::Box;
use alloc::vec::Vec;
use field_path::field_accessor::FieldAccessor;
use hashbrown::HashMap;

use crate::ThreadSafe;
use crate::action::{
    Action, ActionBuilder, ActionClip, ActionId, ActionKey,
    ActionTable, InterpActionBuilder, SampleMode,
};
use crate::interpolation::Interpolation;
use crate::pipeline::{BakeCtx, PipelineKey, Range, SampleCtx};
use crate::registry::Registry;
use crate::subject::SubjectId;
use crate::track::{ClipOverlap, Track, TrackList};
use crate::world::SubjectSource;

pub struct Timeline<W> {
    action_table: ActionTable,
    pipeline_counts: Box<[(PipelineKey, u32)]>,
    /// Track length is guaranteed to be at least 1 by construction.
    /// See [`TimelineBuilder::compile()`].
    tracks: Box<[Track]>,
    /// Cached actions that are queued to be sampled.
    ///
    /// This cache will be cleared everytime [`Timeline::queue_actions`]
    /// is called.
    queue_cache: QueueCache,
    /// Queued actions grouped by pipeline, each carrying its resolved
    /// [`SampleMode`]. Rebuilt from `queue_cache` every
    /// [`Timeline::queue_actions`] so sampling touches only the marked
    /// actions of each type, with no per-action column lookup.
    sample_queue: HashMap<PipelineKey, Vec<(ActionId, SampleMode)>>,
    /// The current time of the current track.
    curr_time: Duration,
    /// The target time of the target track.
    target_time: Duration,
    /// The index of the current track.
    curr_index: usize,
    /// The index of the target track.
    target_index: usize,
    _marker: PhantomData<fn() -> W>,
}

/// Which clip drives a lane at `target`, and how to sample it.
///
/// The one on screen is whichever was authored last among those
/// covering the playhead. When nothing covers it, the lane holds
/// whatever wrote its value last.
///
/// `clips` must be sorted by [`ActionClip::start`] and non-empty.
fn resolve_clip(
    clips: &[ActionClip],
    target: Duration,
) -> (&ActionClip, SampleMode) {
    // Only clips that have begun can matter, and the lane is
    // start-sorted, so they are a prefix. Everything below works off
    // it and can drop the `start` half of its test.
    let started =
        &clips[..clips.partition_point(|clip| clip.start <= target)];

    // Inclusive at the end, so a clip's final instant is still
    // inside it and its easing applies there.
    let covering = started
        .iter()
        .filter(|clip| target <= clip.end())
        .max_by_key(|clip| clip.order);

    if let Some(clip) = covering {
        return (clip, SampleMode::Interp(clip.progress(target)));
    }

    match started.iter().max_by_key(|clip| (clip.end(), clip.order)) {
        Some(clip) => (clip, SampleMode::End),
        None => (&clips[0], SampleMode::Start),
    }
}

impl<W: 'static> Timeline<W> {
    pub fn bake_actions(
        &mut self,
        registry: &Registry,
        subject_world: &W,
    ) {
        for key in self.pipeline_counts.iter().map(|(key, _)| key) {
            for track in self.tracks.iter() {
                let ok = registry.pipeline.bake(
                    key,
                    BakeCtx {
                        world: subject_world,
                        track,
                        action_table: &mut self.action_table,
                        accessor_registry: &registry.accessor,
                    },
                );
                debug_assert!(
                    ok,
                    "pipeline not found for key {key:?}"
                );
            }
        }
    }

    /// Determines which actions are active at the current target time
    /// and marks them for sampling.
    ///
    /// This step is intentionally separate from
    /// [`Self::sample_queued_actions`] so that multiple timelines can
    /// queue concurrently. Queuing only requires `&mut self`, whereas
    /// sampling requires `&mut W`, which would prevent parallel
    /// execution across timelines sharing the same world.
    pub fn queue_actions(&mut self) {
        if self.tracks.is_empty() {
            return;
        }

        self.reset_queues();
        // Current time will change if the track index changes.
        let mut curr_time = self.curr_time();

        // Handle index changes.
        if self.target_index() != self.curr_index() {
            let (sample_mode, track_range) = if self.target_index()
                > self.curr_index()
            {
                // From the start.
                curr_time = Duration::ZERO;
                (
                    SampleMode::End,
                    self.curr_index()..self.target_index(),
                )
            } else {
                // From the end.
                curr_time = self.tracks[self.target_index].duration();
                (
                    SampleMode::Start,
                    (self.target_index() + 1)
                        ..(self.curr_index() + 1),
                )
            };

            for i in track_range {
                let track = &self.tracks[i];

                for (seq, (key, span)) in
                    track.sequences_spans().iter().enumerate()
                {
                    if span.len == 0 {
                        continue;
                    }

                    let clips = track.clips(*span);
                    let clip = match sample_mode {
                        // Only the first stored clip's start holds
                        // the value from before the lane ran.
                        SampleMode::Start => &clips[0],
                        SampleMode::End => {
                            &clips[track.last_to_finish(seq)]
                        }
                        SampleMode::Interp(_) => unreachable!(),
                    };

                    self.queue_cache.cache(
                        *key,
                        clip.id,
                        sample_mode,
                    );
                }
            }

            self.curr_index = self.target_index;
        }

        let time_range = Range {
            start: curr_time.min(self.target_time()),
            end: curr_time.max(self.target_time()),
        };

        let target = self.target_time;
        let track = &self.tracks[self.curr_index];

        for (seq, (key, span)) in
            track.sequences_spans().iter().enumerate()
        {
            if span.len == 0 {
                continue;
            }

            let clips = track.clips(*span);

            // An overlapping clip that began earlier can still
            // finish last, hence `last_to_finish` for the end.
            let clips_range = Range {
                start: clips[0].start,
                end: clips[track.last_to_finish(seq)].end(),
            };

            if !time_range.overlap(&clips_range) {
                continue;
            }

            let (clip, sample_mode) = resolve_clip(clips, target);

            // Without this a finished animation is rewritten every
            // frame. Always true when the clip covers the playhead,
            // so it only filters the held-value cases.
            let clip_range = Range {
                start: clip.start,
                end: clip.end(),
            };
            if !time_range.overlap(&clip_range) {
                continue;
            }

            self.queue_cache.cache(*key, clip.id, sample_mode);
        }

        // Group the deduped queue by pipeline so each typed sampler
        // iterates only its own actions, with the `SampleMode` in hand.
        for (key, &(id, sample_mode)) in self.queue_cache.iter() {
            let pkey = PipelineKey::from_action_key::<W>(*key);
            self.sample_queue
                .entry(pkey)
                .or_default()
                .push((id, sample_mode));
        }

        self.curr_time = self.target_time;
    }

    pub fn sample_queued_actions(
        &self,
        registry: &Registry,
        subject_world: &mut W,
    ) {
        for (key, samples) in self.sample_queue.iter() {
            if samples.is_empty() {
                continue;
            }
            let ok = registry.pipeline.sample(
                key,
                SampleCtx {
                    world: subject_world,
                    action_table: &self.action_table,
                    accessor_registry: &registry.accessor,
                    samples,
                },
            );
            debug_assert!(ok, "pipeline not found for key {key:?}");
        }
    }

    fn reset_queues(&mut self) {
        self.queue_cache.clear();
        // Retain the per-pipeline `Vec` capacities across frames.
        for samples in self.sample_queue.values_mut() {
            samples.clear();
        }
    }
}

// Getter methods.
impl<W> Timeline<W> {
    /// Every [`ClipOverlap`] found while compiling this timeline's
    /// tracks. Empty when nothing overlaps.
    pub fn overlaps(&self) -> impl Iterator<Item = &ClipOverlap> {
        self.tracks.iter().flat_map(|track| track.overlaps())
    }

    /// Returns the current queue cache.
    #[inline]
    pub fn queue_cache(&self) -> &QueueCache {
        &self.queue_cache
    }

    /// Returns the current playback time.
    #[inline]
    pub fn curr_time(&self) -> Duration {
        self.curr_time
    }

    /// Returns the target playback time.
    #[inline]
    pub fn target_time(&self) -> Duration {
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

    /// Returns a reference slice to all tracks.
    #[inline]
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// Returns a reference the current playing track.
    #[inline]
    pub fn curr_track(&self) -> &Track {
        // SAFETY: Track length is garuanteed to be at least 1.
        &self.tracks[self.curr_index]
    }

    /// Get the index of the last track. This is essentially the largest
    /// index you can provide in [`Timeline::set_target_track`].
    #[inline]
    pub fn last_track_index(&self) -> usize {
        // SAFETY: Track length is garuanteed to be at least 1.
        self.tracks.len() - 1
    }

    /// Returns `true` if the current track is the last track.
    #[inline]
    pub fn is_last_track(&self) -> bool {
        self.curr_index == self.last_track_index()
    }

    /// Has [`Self::curr_time()`] reached the end of the track at
    /// [`Self::curr_index()`]?
    #[inline]
    pub fn is_track_end(&self) -> bool {
        // SAFETY: Track length is garuanteed to be at least 1.
        self.curr_time >= self.tracks[self.curr_index()].duration()
    }

    /// Is [`Self::is_last_track()`] and [`Self::is_track_end()`].
    #[inline]
    pub fn is_complete(&self) -> bool {
        self.is_last_track() && self.is_track_end()
    }
}

// Setter methods.
impl<W> Timeline<W> {
    /// Set the target time of the current track, clamping the value
    /// within \[0.0..=track.duration\]
    pub fn set_target_time(
        &mut self,
        target_time: Duration,
    ) -> &mut Self {
        let duration = self.tracks[self.target_index].duration();

        self.target_time = target_time.min(duration);
        self
    }

    /// Steps forward, clamping at the track's end.
    pub fn advance_time(&mut self, time: Duration) -> &mut Self {
        let target_time = self.target_time.saturating_add(time);

        self.set_target_time(target_time)
    }

    /// Steps backward, saturating at [`Duration::ZERO`].
    ///
    /// [`Duration`] carries no sign, hence a separate method.
    pub fn rewind_time(&mut self, time: Duration) -> &mut Self {
        let target_time = self.target_time.saturating_sub(time);

        self.set_target_time(target_time)
    }

    /// Set the target track index, clamping the value within
    /// \[0..=track_count - 1\].
    pub fn set_target_track(
        &mut self,
        target_index: usize,
    ) -> &mut Self {
        let max_index = self.last_track_index();

        self.target_index = target_index.clamp(0, max_index);
        self
    }
}

/// Cached actions that are queued to be sampled.
///
/// This cache prevents duplicated samples on the same [`ActionKey`]
/// which result in sampling the same target field on the same entity
/// more than once. This is crucial as the sampling pipeline happens
/// in an unordered manner.
#[derive(Debug)]
pub struct QueueCache {
    cache: HashMap<ActionKey, (ActionId, SampleMode)>,
}

impl QueueCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&ActionKey, &(ActionId, SampleMode))>
    {
        self.cache.iter()
    }

    pub fn iter_keys(&self) -> impl Iterator<Item = &ActionKey> {
        self.cache.keys()
    }

    pub fn iter_ids(&self) -> impl Iterator<Item = ActionId> + '_ {
        self.cache.values().map(|(id, _)| *id)
    }

    /// Clear all the cached contents.
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Cache an [`ActionKey`] with its [`SampleMode`], overwriting any
    /// previous entry for the same key (dedup per field per subject).
    pub fn cache(
        &mut self,
        key: ActionKey,
        id: ActionId,
        sample_mode: SampleMode,
    ) {
        self.cache.insert(key, (id, sample_mode));
    }
}

impl Default for QueueCache {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TimelineBuilder<'a, W> {
    registry: &'a mut Registry,
    action_table: ActionTable,
    pipeline_counts: HashMap<PipelineKey, u32>,
    _marker: PhantomData<fn() -> W>,
}

impl<'a, W: 'static> TimelineBuilder<'a, W> {
    /// Creates an empty timeline builder.
    pub fn new(registry: &'a mut Registry) -> Self {
        Self {
            registry,
            action_table: ActionTable::new(),
            pipeline_counts: HashMap::new(),
            _marker: PhantomData,
        }
    }

    /// Access the underlying runtime registry (needed by the scene
    /// compile step to look up typed accessors).
    pub fn registry(&self) -> &Registry {
        self.registry
    }

    /// Add an [`Action`] with interpolation using
    /// [`Interpolation::interp`].
    pub fn act<I, S, T, M>(
        &mut self,
        target: I,
        field_acc: FieldAccessor<S, T>,
        action: impl Action<T>,
    ) -> InterpActionBuilder<'_, T>
    where
        W: SubjectSource<I, S> + 'static,
        I: SubjectId,
        S: 'static,
        T: Interpolation<M> + Clone + ThreadSafe,
    {
        self.act_builder(target, field_acc, action)
            .with_interp(T::interp)
    }

    /// Add an [`Action`] using step interpolation.
    pub fn act_step<I, S, T>(
        &mut self,
        target: I,
        field_acc: FieldAccessor<S, T>,
        action: impl Action<T>,
    ) -> InterpActionBuilder<'_, T>
    where
        W: SubjectSource<I, S> + 'static,
        I: SubjectId,
        S: 'static,
        T: Clone + ThreadSafe,
    {
        self.act_builder(target, field_acc, action).with_interp(
            |a, b, t| {
                if t < 1.0 { a.clone() } else { b.clone() }
            },
        )
    }

    /// Add an [`Action`] without interpolation, returning an
    /// [`ActionBuilder`] for manual configuration.
    pub fn act_builder<I, S, T>(
        &mut self,
        target: I,
        field_acc: FieldAccessor<S, T>,
        action: impl Action<T>,
    ) -> ActionBuilder<'_, T>
    where
        W: SubjectSource<I, S> + 'static,
        I: SubjectId,
        S: 'static,
        T: Clone + ThreadSafe,
    {
        let field = field_acc.field;
        self.registry.register::<W, I, S, T>(field_acc);
        let key = PipelineKey::new::<W, I, S, T>();

        match self.pipeline_counts.get_mut(&key) {
            Some(count) => *count += 1,
            None => {
                self.pipeline_counts.insert(key, 1);
            }
        }

        self.action_table.add(target, field, action)
    }

    /// Remove an [`Action`].
    pub fn unact(&mut self, id: ActionId) -> bool {
        if let Some(key) = self.action_table.remove(id) {
            let pipeline_key = PipelineKey::from_action_key::<W>(key);

            let count = self
                .pipeline_counts
                .get_mut(&pipeline_key)
                .unwrap_or_else(|| {
                    panic!(
                        "Field counts not registered for {:?}!",
                        key.field()
                    )
                });

            *count -= 1;
            if *count == 0 {
                self.pipeline_counts.remove(&pipeline_key);
            }

            return true;
        }

        false
    }

    /// Compile into a [`Timeline`].
    pub fn compile(
        self,
        tracks: impl Into<TrackList>,
    ) -> Timeline<W> {
        Timeline {
            action_table: self.action_table,
            pipeline_counts: self
                .pipeline_counts
                .into_iter()
                .collect(),
            tracks: tracks.into().into_boxed_slice(),
            queue_cache: QueueCache::new(),
            sample_queue: HashMap::new(),
            curr_time: Duration::ZERO,
            target_time: Duration::ZERO,
            curr_index: 0,
            target_index: 0,
            _marker: PhantomData,
        }
    }
}

// TODO: Write some unit tests.
#[cfg(test)]
mod tests {
    use crate::registry::Registry;
    use crate::time::cs;
    use crate::track::{TrackOrdering, delay};
    use crate::world::SubjectSource;

    use super::*;

    /// A handful of `f32` subjects, addressed by index.
    struct World([f32; 2]);

    impl World {
        fn new() -> Self {
            Self([0.0; 2])
        }
    }

    impl SubjectSource<u32, f32> for World {
        fn get_source(&self, id: u32) -> Option<&f32> {
            self.0.get(id as usize)
        }

        fn apply_source<R>(
            &mut self,
            id: u32,
            f: impl FnOnce(&mut f32) -> R,
        ) -> Option<R> {
            self.0.get_mut(id as usize).map(f)
        }
    }

    fn linear(a: &f32, b: &f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    /// Builds a timeline from fragments authored in the given order,
    /// each moving `x` one unit to the right.
    ///
    /// `spans` is `(start, duration)` in centiseconds; authoring order
    /// is the order they are listed.
    fn timeline_of(
        spans: &[(u64, u64)],
    ) -> (Registry, Timeline<World>) {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<World>();

        let mut fragments = Vec::new();
        for (start, duration) in spans {
            let fragment = builder
                .act_builder(0u32, crate::path!(<f32>), |x| x + 1.0)
                .with_interp(linear)
                .play(cs(*duration));

            fragments.push(delay(cs(*start), fragment));
        }

        let track = fragments.ord_all().compile();
        let timeline = builder.compile(track);

        (registry, timeline)
    }

    /// Plays to `time` and returns what subject 0 holds.
    fn sample_at(
        registry: &Registry,
        timeline: &mut Timeline<World>,
        world: &mut World,
        time: Duration,
    ) -> f32 {
        timeline.set_target_time(time);
        timeline.queue_actions();
        timeline.sample_queued_actions(registry, world);

        world.0[0]
    }

    /// The clip authored last wins while it covers the playhead, and
    /// the one underneath shows again once it passes — with no resume
    /// logic, because nothing is overwriting it any more.
    #[test]
    fn later_clip_overrides_then_the_earlier_one_resumes() {
        // 0..5s authored first, 1..2s authored second.
        let (registry, mut timeline) =
            timeline_of(&[(0, 500), (100, 100)]);

        // Reachable from the timeline, not just the track.
        assert_eq!(timeline.overlaps().count(), 1);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Only the long clip covers 0.5s.
        let before =
            sample_at(&registry, &mut timeline, &mut world, cs(50));
        assert!((before - 0.1).abs() < 1e-5, "got {before}");

        // Both cover 1.5s; the one authored later wins. Its segment
        // is chained off the first one's end, so it runs 1.0 -> 2.0.
        let during =
            sample_at(&registry, &mut timeline, &mut world, cs(150));
        assert!((during - 1.5).abs() < 1e-5, "got {during}");

        // Past the override, the long clip is on screen again at its
        // own progress.
        let after =
            sample_at(&registry, &mut timeline, &mut world, cs(250));
        assert!((after - 0.5).abs() < 1e-5, "got {after}");

        // Arriving at 1.5s from the right gives the same answer:
        // scrubbing resolves the winner independently of direction.
        let back =
            sample_at(&registry, &mut timeline, &mut world, cs(150));
        assert!((back - 1.5).abs() < 1e-5, "got {back}");
    }

    /// Which clip wins is decided by authoring order, not by timing.
    ///
    /// Authoring the short clip *first* puts it underneath the long
    /// one for the whole of its life, so it never plays at all and is
    /// removed — leaving the long clip animating at its own values
    /// rather than chained off a clip nobody can see.
    #[test]
    fn authoring_order_decides_the_winner() {
        // The same two spans as above, authored the other way round.
        let (registry, mut timeline) =
            timeline_of(&[(100, 100), (0, 500)]);

        assert_eq!(timeline.overlaps().count(), 1);
        assert!(
            timeline.overlaps().next().unwrap().never_visible,
            "the short clip is covered for its entire span"
        );

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Only the long clip is left: 0.0 -> 1.0 across 5s, so 30%
        // of the way at 1.5s.
        let during =
            sample_at(&registry, &mut timeline, &mut world, cs(150));
        assert!((during - 0.3).abs() < 1e-5, "got {during}");
    }

    /// With no overlap the winner lookup must agree with the plain
    /// one-clip-per-instant case.
    #[test]
    fn non_overlapping_clips_play_in_sequence() {
        let (registry, mut timeline) =
            timeline_of(&[(0, 100), (100, 100)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        let first =
            sample_at(&registry, &mut timeline, &mut world, cs(50));
        assert!((first - 0.5).abs() < 1e-5, "got {first}");

        let second =
            sample_at(&registry, &mut timeline, &mut world, cs(150));
        assert!((second - 1.5).abs() < 1e-5, "got {second}");
    }

    /// A playhead parked in a gap holds the value the last clip left
    /// behind, rather than jumping to one that has not started.
    #[test]
    fn gap_holds_the_previous_clip_end() {
        // 0..1s, then nothing until 5..6s.
        let (registry, mut timeline) =
            timeline_of(&[(0, 100), (500, 100)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        let in_gap =
            sample_at(&registry, &mut timeline, &mut world, cs(300));
        assert!((in_gap - 1.0).abs() < 1e-5, "got {in_gap}");
    }

    /// With several clips already finished, the gap holds the one
    /// that finished *last*, not the first to have started.
    ///
    /// The test above cannot see the difference: only one clip has
    /// started by the time it samples.
    #[test]
    fn gap_holds_the_last_clip_to_finish_not_the_first() {
        // 0..1s, 2..3s, then 5..6s to keep the track running.
        let (registry, mut timeline) =
            timeline_of(&[(0, 100), (200, 100), (500, 100)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // At 4s the first two have finished and the third has not
        // begun. Baking chains 0 -> 1 -> 2, so the second one's end
        // is 2.0; taking the first would give 1.0.
        let in_gap =
            sample_at(&registry, &mut timeline, &mut world, cs(400));
        assert!((in_gap - 2.0).abs() < 1e-5, "got {in_gap}");
    }

    /// Past the end of everything, the subject holds the value of the
    /// clip that finished last — which with overlaps is not the last
    /// clip in the span.
    #[test]
    fn past_the_end_holds_the_last_clip_to_finish() {
        // 0..10s authored first, 1..2s authored second. Sorted by
        // start the short clip is second, but the long one outlives
        // it, so it owns the resting value.
        let (registry, mut timeline) =
            timeline_of(&[(0, 1000), (100, 100)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        let resting =
            sample_at(&registry, &mut timeline, &mut world, cs(1000));
        assert!((resting - 1.0).abs() < 1e-5, "got {resting}");
    }

    /// Two clips starting together where the later one is shorter:
    /// it wins while it lasts, then the first shows through its tail.
    /// Neither is fully hidden, so both survive.
    #[test]
    fn shared_start_with_a_visible_tail_keeps_both() {
        // 0..3s authored first, 0..2s authored second.
        let (registry, mut timeline) =
            timeline_of(&[(0, 300), (0, 200)]);

        assert_eq!(timeline.overlaps().count(), 1);
        assert!(!timeline.overlaps().next().unwrap().never_visible);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Both cover 1s; the one authored second wins. It bakes
        // 1.0 -> 2.0 and is halfway through.
        let covered =
            sample_at(&registry, &mut timeline, &mut world, cs(100));
        assert!((covered - 1.5).abs() < 1e-5, "got {covered}");

        // Only the first clip reaches 2.5s: 0.0 -> 1.0 across 3s.
        let tail =
            sample_at(&registry, &mut timeline, &mut world, cs(250));
        assert!((tail - 0.8333).abs() < 1e-3, "got {tail}");
    }

    /// A covering clip does not continue from what is on screen: it
    /// starts from the *end* value of the clip it covers, because
    /// baking chains clips linearly. The jump at 3s is to 1.0 — a
    /// value the lane never displayed.
    ///
    /// Documented rather than asserted as desirable. See the
    /// "linear bake chain" note in `OVERLAP_HANDLING.md`.
    #[test]
    fn overlap_start_jumps_to_the_covered_clips_end_value() {
        // A = 0..5s authored first, B = 3..5s authored second.
        let (registry, mut timeline) =
            timeline_of(&[(0, 500), (300, 200)]);
        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        let before =
            sample_at(&registry, &mut timeline, &mut world, cs(299));
        assert!((before - 0.598).abs() < 1e-3, "got {before}");

        // Not 0.6: B was baked from A's end, not from A at 3s.
        let after =
            sample_at(&registry, &mut timeline, &mut world, cs(300));
        assert!((after - 1.0).abs() < 1e-5, "got {after}");
    }

    /// Scrubbing back behind a lane holds the value from *before* any
    /// of its clips ran, not a partially applied chain.
    ///
    /// Baking reads the subject's own value into the first stored
    /// clip and chains from there, so only that clip's start is
    /// untouched. A lane whose clips share the earliest start makes
    /// the difference visible: the later-authored one wins *at* 5s,
    /// but before 5s nothing has run at all.
    #[test]
    fn before_a_shared_start_holds_the_untouched_value() {
        // Both begin at 5s: 5..8s authored first, 5..7s second.
        let (registry, mut timeline) =
            timeline_of(&[(500, 300), (500, 200)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Arrive from inside the lane so scrubbing back reaches the
        // branch that holds the pre-lane value.
        sample_at(&registry, &mut timeline, &mut world, cs(600));

        // Baking walks storage order: first 0.0 -> 1.0, then second
        // 1.0 -> 2.0. Taking the later-authored clip here would show
        // 1.0 — the first clip's end, before the first clip has run.
        let before =
            sample_at(&registry, &mut timeline, &mut world, cs(0));
        assert!(before.abs() < 1e-5, "got {before}");
    }

    /// Two zero-duration clips at one instant do not *overlap* by
    /// [`ActionClip::overlaps`], yet both are live there, so the
    /// winner is still decided by `order`.
    ///
    /// Storage order is deliberately the reverse of authoring order.
    /// Anything that resolves a lane by position rather than `order`
    /// picks wrong here.
    #[test]
    fn zero_duration_spacers_resolve_by_order_not_position() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<World>();

        // Authored first, so the lower `order`.
        let first = builder
            .act_builder(0u32, crate::path!(<f32>), |x| x + 1.0)
            .with_interp(linear)
            .play(Duration::ZERO);
        // Authored second, so the higher `order`: this must win.
        let second = builder
            .act_builder(0u32, crate::path!(<f32>), |x| x + 5.0)
            .with_interp(linear)
            .play(Duration::ZERO);

        // Listed the other way round, so `second` is stored first.
        let track = [second, first].ord_all().compile();
        let mut timeline = builder.compile(track);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Baking walks storage order: `second` 0 -> 5, then `first`
        // 5 -> 6. The higher-order clip is `second`, so 5.0. Picking
        // by position would give 6.0.
        let at_zero = sample_at(
            &registry,
            &mut timeline,
            &mut world,
            Duration::ZERO,
        );
        assert!((at_zero - 5.0).abs() < 1e-5, "got {at_zero}");
    }

    /// A zero-duration spacer sitting exactly where a longer clip
    /// ends. Their spans do not overlap, but both are live at that
    /// instant, so `order` still decides.
    ///
    /// Picking the later clip by position is *usually* harmless at a
    /// touching boundary, since baking sets its start to the earlier
    /// one's end. Not here: `progress` reports `1.0` for a
    /// zero-duration clip, so it reads as its own *end* instead.
    #[test]
    fn zero_duration_clip_at_a_boundary_resolves_by_order() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<World>();

        // Authored first, so the lower `order`.
        let spacer = delay(
            cs(500),
            builder
                .act_builder(0u32, crate::path!(<f32>), |x| x + 5.0)
                .with_interp(linear)
                .play(Duration::ZERO),
        );
        // Authored second, so the higher `order`: this must win.
        let long = builder
            .act_builder(0u32, crate::path!(<f32>), |x| x + 1.0)
            .with_interp(linear)
            .play(cs(500));

        let track = [spacer, long].ord_all().compile();
        let mut timeline = builder.compile(track);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Baking walks start order: long 0 -> 1, then spacer 1 -> 6.
        // At 5s both are live and the long clip was authored later,
        // so it holds its own end. Picking by position gives 6.0.
        let at_end =
            sample_at(&registry, &mut timeline, &mut world, cs(500));
        assert!((at_end - 1.0).abs() < 1e-5, "got {at_end}");
    }

    /// Three clips over one instant: the last authored wins outright.
    #[test]
    fn three_way_overlap_resolves_to_the_last_authored() {
        // All three cover 1.5s; the third bakes 2.0 -> 3.0.
        let (registry, mut timeline) =
            timeline_of(&[(0, 500), (100, 400), (100, 300)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        let during =
            sample_at(&registry, &mut timeline, &mut world, cs(150));
        assert!(
            (during - 2.1667).abs() < 1e-3,
            "the last authored clip should win, got {during}"
        );
    }

    /// Scrubbing back before a lane begins resolves it to its opening
    /// value rather than leaving whatever was last written.
    #[test]
    fn before_the_start_holds_the_opening_value() {
        // A single clip running 5..6s.
        let (registry, mut timeline) = timeline_of(&[(500, 100)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        let midway =
            sample_at(&registry, &mut timeline, &mut world, cs(550));
        assert!((midway - 0.5).abs() < 1e-5, "got {midway}");

        // Back before it starts: the clip's opening value, not its
        // end and not whatever the playhead left behind.
        let before =
            sample_at(&registry, &mut timeline, &mut world, cs(0));
        assert!((before - 0.0).abs() < 1e-5, "got {before}");
    }

    /// A playhead that stops moving must not rewrite finished clips
    /// every frame. The `time_range` guard is what prevents it.
    #[test]
    fn parked_playhead_stops_requeueing() {
        // A gap to park in. A single clip would not do: the target
        // time is clamped to the track duration, which would leave
        // the playhead sitting exactly on the clip's end — where it
        // still counts as covered.
        let (registry, mut timeline) =
            timeline_of(&[(0, 100), (500, 100)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Crossing the first clip queues it, to land on its end.
        sample_at(&registry, &mut timeline, &mut world, cs(300));
        assert!(!timeline.queue_cache().is_empty());

        // Sitting still in the gap moves through nothing, so nothing
        // is queued.
        sample_at(&registry, &mut timeline, &mut world, cs(300));
        assert!(
            timeline.queue_cache().is_empty(),
            "a parked playhead requeued a finished clip"
        );
    }

    /// Skipping *backwards* past a track resolves its lanes to the
    /// value they held before any of their clips ran.
    ///
    /// Baking chains from the first stored clip, so only that clip's
    /// start is the untouched value. The clip that finishes last is
    /// the wrong end of the lane, and with overlaps it is a different
    /// clip entirely.
    #[test]
    fn skipping_a_lane_backwards_holds_its_pre_lane_value() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<World>();

        // Track 0 is only somewhere to jump back to.
        let track0 = builder
            .act_builder(0u32, crate::path!(<f32>), |x| x + 1.0)
            .with_interp(linear)
            .play(cs(100))
            .compile();

        // Track 1's lane: 0..1s, plus 0.5..3s overlapping it. Sorted
        // by start, so the first stored clip is *not* the one that
        // finishes last.
        let short = builder
            .act_builder(1u32, crate::path!(<f32>), |x| x + 1.0)
            .with_interp(linear)
            .play(cs(100));
        let long = delay(
            cs(50),
            builder
                .act_builder(1u32, crate::path!(<f32>), |x| x + 3.0)
                .with_interp(linear)
                .play(cs(250)),
        );
        let track1 = [short, long].ord_all().compile();

        let tracks = TrackList::collect([track0, track1]).unwrap();
        let mut timeline = builder.compile(tracks);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Play track 1 out so subject 1 is far from where it began.
        timeline.set_target_track(1);
        timeline.set_target_time(cs(300));
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);
        assert!(world.0[1] > 1.0, "got {}", world.0[1]);

        // Jump back. Track 1 is the skipped one, so its lane settles
        // to 0.0. Taking the clip that finishes last would give 1.0,
        // the short clip's end — a value from mid-lane.
        timeline.set_target_track(0);
        timeline.set_target_time(Duration::ZERO);
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);

        assert!(world.0[1].abs() < 1e-5, "got {}", world.0[1]);
    }

    /// Skipping tracks resolves each skipped lane to its resting
    /// value, and that value belongs to the clip finishing last —
    /// not to the last clip in the span.
    #[test]
    fn track_skip_uses_the_clip_that_finishes_last() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<World>();

        // Track 0 drives subject 0 with an overlapping pair: 0..10s
        // authored first, 1..2s authored second.
        let long = builder
            .act_builder(0u32, crate::path!(<f32>), |x| x + 1.0)
            .with_interp(linear)
            .play(cs(1000));
        let short = delay(
            cs(100),
            builder
                .act_builder(0u32, crate::path!(<f32>), |x| x + 5.0)
                .with_interp(linear)
                .play(cs(100)),
        );
        let track0 = [long, short].ord_all().compile();

        // Track 1 drives a different subject, so nothing overwrites
        // subject 0 once we arrive.
        let track1 = builder
            .act_builder(1u32, crate::path!(<f32>), |x| x + 1.0)
            .with_interp(linear)
            .play(cs(100))
            .compile();

        let tracks = TrackList::collect([track0, track1]).unwrap();
        let mut timeline = builder.compile(tracks);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Jump forward a track: track 0's lane resolves to its end.
        timeline.set_target_track(1);
        timeline.set_target_time(Duration::ZERO);
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);

        // The long clip finishes last, so it owns the resting value
        // (0.0 -> 1.0). Taking the short clip instead would leave 6.0.
        assert!(
            (world.0[0] - 1.0).abs() < 1e-5,
            "got {}",
            world.0[0]
        );

        // Play track 1 out, so the backward jump has something to
        // undo on the lane it will skip.
        timeline.set_target_time(cs(100));
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);
        assert!(
            (world.0[1] - 1.0).abs() < 1e-5,
            "got {}",
            world.0[1]
        );

        timeline.set_target_track(0);
        timeline.set_target_time(Duration::ZERO);
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);

        // Track 1 is the *skipped* lane on the way back, so the
        // transition path settles it to its opening value.
        assert!(
            (world.0[1] - 0.0).abs() < 1e-5,
            "got {}",
            world.0[1]
        );
        // Track 0 is the target, so its lane comes from the main
        // loop at progress 0 — not from the transition path.
        assert!(
            (world.0[0] - 0.0).abs() < 1e-5,
            "got {}",
            world.0[0]
        );
    }
}
