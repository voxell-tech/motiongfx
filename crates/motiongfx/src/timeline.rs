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
use crate::track::{Track, TrackList};
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

/// Which clip of a lane is on screen at `target`, as an offset into
/// `clips`, and how to sample it.
///
/// The one on screen is whichever comes last in the lane among those
/// covering `target`. When nothing covers it, the lane holds whatever
/// wrote its value last.
///
/// `clips` is a lane as [`Track::clips`] hands it back: sorted by
/// [`ActionClip::start`] and non-empty.
pub(crate) fn resolve_clip(
    clips: &[ActionClip],
    target: Duration,
) -> (usize, SampleMode) {
    // Only clips that have begun can matter, and the lane is
    // start-sorted, so they are a prefix. Everything below works off
    // it and can drop the `start` half of its test.
    let started =
        &clips[..clips.partition_point(|clip| clip.start <= target)];

    // Clips later in the lane win, so scan backwards and stop at the
    // first that is still running.
    if let Some(i) =
        started.iter().rposition(|clip| target <= clip.end())
    {
        return (i, SampleMode::Interp(clips[i].progress(target)));
    }

    match started
        .iter()
        .enumerate()
        .max_by_key(|(_, clip)| clip.end())
    {
        Some((i, _)) => (i, SampleMode::End),
        None => (0, SampleMode::Start),
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

                for (key, span) in track.sequences_spans().iter() {
                    if span.len == 0 {
                        continue;
                    }

                    let clips = track.clips(*span);
                    let clip = match sample_mode {
                        // Only the first stored clip's start holds
                        // the value from before the lane ran.
                        SampleMode::Start => &clips[0],
                        SampleMode::End => clips
                            .iter()
                            .max_by_key(|clip| clip.end())
                            .expect("a lane is never empty"),
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

        for (key, span) in track.sequences_spans().iter() {
            if span.len == 0 {
                continue;
            }

            let clips = track.clips(*span);

            let (i, sample_mode) = resolve_clip(clips, target);
            let clip = &clips[i];

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

    /// Builds a timeline from fragments in the given order,
    /// each moving `x` one unit to the right.
    ///
    /// `spans` is `(start, duration)` in centiseconds. Lanes are
    /// stored sorted by start, ties keeping list order.
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

    /// The later clip wins while it covers, the one underneath shows
    /// again after. The two handovers are not symmetric: taking over
    /// is seamless, handing back jumps — a clip has one segment, so
    /// it cannot be reopened part way through.
    #[test]
    fn later_clip_overrides_then_the_earlier_one_resumes() {
        // 0..5s, with 1..2s over it. The override runs 0.2 -> 1.2,
        // opening on what the long clip shows at 1s.
        let (registry, mut timeline) =
            timeline_of(&[(0, 500), (100, 100)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        let before =
            sample_at(&registry, &mut timeline, &mut world, cs(50));
        assert!((before - 0.1).abs() < 1e-5, "got {before}");

        let during =
            sample_at(&registry, &mut timeline, &mut world, cs(150));
        assert!((during - 0.7).abs() < 1e-5, "got {during}");

        // The override's final instant, then the jump back.
        let last =
            sample_at(&registry, &mut timeline, &mut world, cs(200));
        assert!((last - 1.2).abs() < 1e-5, "got {last}");

        let resumed =
            sample_at(&registry, &mut timeline, &mut world, cs(201));
        assert!((resumed - 0.402).abs() < 1e-3, "got {resumed}");

        let after =
            sample_at(&registry, &mut timeline, &mut world, cs(250));
        assert!((after - 0.5).abs() < 1e-5, "got {after}");

        // Scrubbing back gives the same winner.
        let back =
            sample_at(&registry, &mut timeline, &mut world, cs(150));
        assert!((back - 0.7).abs() < 1e-5, "got {back}");
    }

    /// Which clip wins is decided by position in the lane, and lanes
    /// are stored sorted by start — so listing the fragments the other
    /// way round cannot change the answer.
    #[test]
    fn creation_order_does_not_decide_the_winner() {
        // The same two spans as above, created the other way round.
        let (registry, mut timeline) =
            timeline_of(&[(100, 100), (0, 500)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        let during =
            sample_at(&registry, &mut timeline, &mut world, cs(150));
        assert!((during - 0.7).abs() < 1e-5, "got {during}");
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

    /// A playhead parked in a gap holds the clip that finished
    /// *last*, not the first to have started.
    #[test]
    fn gap_holds_the_last_clip_to_finish_not_the_first() {
        // 0..1s, 2..3s, then 5..6s to keep the track running.
        let (registry, mut timeline) =
            timeline_of(&[(0, 100), (200, 100), (500, 100)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // At 4s two have finished and the third has not begun. The
        // chain is 0 -> 1 -> 2, so the answer is 2.0; the first clip
        // would give 1.0.
        let in_gap =
            sample_at(&registry, &mut timeline, &mut world, cs(400));
        assert!((in_gap - 2.0).abs() < 1e-5, "got {in_gap}");
    }

    /// Past the end, the subject holds the clip that finished last —
    /// with overlaps, not the last clip in the lane.
    #[test]
    fn past_the_end_holds_the_last_clip_to_finish() {
        // 0..10s with 1..2s over it: the short clip is stored second
        // but the long one outlives it.
        let (registry, mut timeline) =
            timeline_of(&[(0, 1000), (100, 100)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        let resting =
            sample_at(&registry, &mut timeline, &mut world, cs(1000));
        assert!((resting - 1.0).abs() < 1e-5, "got {resting}");
    }

    /// Clips starting together: the later one wins while it lasts,
    /// then the first shows through its tail.
    #[test]
    fn shared_start_with_a_visible_tail_keeps_both() {
        // 0..3s and 0..2s. Sharing a start, the second opens where
        // the first did (0.0) and bakes 0.0 -> 1.0.
        let (registry, mut timeline) =
            timeline_of(&[(0, 300), (0, 200)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        let covered =
            sample_at(&registry, &mut timeline, &mut world, cs(100));
        assert!((covered - 0.5).abs() < 1e-5, "got {covered}");

        // Only the first clip reaches 2.5s: 0.0 -> 1.0 across 3s.
        let tail =
            sample_at(&registry, &mut timeline, &mut world, cs(250));
        assert!((tail - 0.8333).abs() < 1e-3, "got {tail}");
    }

    /// A covering clip opens on the value already on screen, so
    /// taking over is continuous. Chaining off the covered clip's end
    /// opened it on 1.0 — a value the lane never displayed.
    #[test]
    fn an_overlap_takes_over_without_a_jump() {
        // 0..5s with 3..5s over it.
        let (registry, mut timeline) =
            timeline_of(&[(0, 500), (300, 200)]);
        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Last instant of the first clip, first of the second.
        let before =
            sample_at(&registry, &mut timeline, &mut world, cs(299));
        assert!((before - 0.598).abs() < 1e-3, "got {before}");

        let after =
            sample_at(&registry, &mut timeline, &mut world, cs(300));
        assert!((after - 0.6).abs() < 1e-5, "got {after}");
        assert!(
            (after - before).abs() < 1e-2,
            "jumped {before} -> {after}"
        );

        // It still animates its own range: 0.6 -> 1.6.
        let end =
            sample_at(&registry, &mut timeline, &mut world, cs(500));
        assert!((end - 1.6).abs() < 1e-5, "got {end}");
    }

    /// Baking walks every lane of a track reusing one set of scratch
    /// buffers, so a lane must open on its own subject rather than on
    /// what the previous lane left behind.
    #[test]
    fn lanes_do_not_leak_into_each_other() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<World>();

        // Two subjects, one track, disjoint spans — so a leak reads
        // the wrong lane rather than coinciding with the right value.
        let first = builder
            .act_builder(0u32, crate::path!(<f32>), |x| x + 10.0)
            .with_interp(linear)
            .play(cs(100));
        let second = delay(
            cs(500),
            builder
                .act_builder(1u32, crate::path!(<f32>), |x| x + 1.0)
                .with_interp(linear)
                .play(cs(100)),
        );

        let track = [first, second].ord_all().compile();
        let mut timeline = builder.compile(track);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Subject 0 rests on its end; subject 1 is half way through
        // its own 0.0 -> 1.0.
        let held =
            sample_at(&registry, &mut timeline, &mut world, cs(550));
        assert!((held - 10.0).abs() < 1e-5, "got {held}");
        assert!(
            (world.0[1] - 0.5).abs() < 1e-5,
            "got {}",
            world.0[1]
        );
    }

    /// Only the first stored clip opens on the untouched value, so
    /// the pre-lane branch must name `clips[0]` specifically.
    #[test]
    fn before_an_overlapping_lane_holds_the_untouched_value() {
        // 5..6s then 5.5..8s, so the second clip is both last in the
        // lane and last to finish — neither is `clips[0]`.
        let (registry, mut timeline) =
            timeline_of(&[(500, 100), (550, 250)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Arrive from inside, so scrubbing back reaches the branch.
        let inside =
            sample_at(&registry, &mut timeline, &mut world, cs(600));
        assert!((inside - 0.7).abs() < 1e-5, "got {inside}");

        // Either other candidate would read 0.5 — a value from the
        // middle of a lane that has not started.
        let before =
            sample_at(&registry, &mut timeline, &mut world, cs(0));
        assert!(before.abs() < 1e-5, "got {before}");
    }

    /// Two spacers at one instant occupy no time, yet both are live
    /// there — position in the lane still decides.
    #[test]
    fn zero_duration_spacers_resolve_by_position() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<World>();

        let first = builder
            .act_builder(0u32, crate::path!(<f32>), |x| x + 1.0)
            .with_interp(linear)
            .play(Duration::ZERO);
        let second = builder
            .act_builder(0u32, crate::path!(<f32>), |x| x + 5.0)
            .with_interp(linear)
            .play(Duration::ZERO);

        // Listed the other way round, so `second` is stored first and
        // bakes 0 -> 5. `first` then opens on what the lane shows at
        // 0s — `progress` is 1.0 for a spacer, so that is `second`'s
        // end — giving 5 -> 6. `first` is stored last, so it wins.
        let track = [second, first].ord_all().compile();
        let mut timeline = builder.compile(track);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        let at_zero = sample_at(
            &registry,
            &mut timeline,
            &mut world,
            Duration::ZERO,
        );
        assert!((at_zero - 6.0).abs() < 1e-5, "got {at_zero}");
    }

    /// A spacer sitting exactly where a longer clip ends. Touching is
    /// usually harmless, since the later clip opens on what the
    /// earlier shows there — but `progress` is `1.0` for a spacer, so
    /// it reads as its own *end* instead.
    #[test]
    fn zero_duration_clip_at_a_boundary_resolves_by_position() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<World>();

        let spacer = delay(
            cs(500),
            builder
                .act_builder(0u32, crate::path!(<f32>), |x| x + 5.0)
                .with_interp(linear)
                .play(Duration::ZERO),
        );
        let long = builder
            .act_builder(0u32, crate::path!(<f32>), |x| x + 1.0)
            .with_interp(linear)
            .play(cs(500));

        let track = [spacer, long].ord_all().compile();
        let mut timeline = builder.compile(track);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Start order puts long first: 0 -> 1, then spacer 1 -> 6.
        // At 5s both are live and the spacer is stored later.
        let at_end =
            sample_at(&registry, &mut timeline, &mut world, cs(500));
        assert!((at_end - 6.0).abs() < 1e-5, "got {at_end}");
    }

    /// Three clips over one instant: the last in the lane wins
    /// outright, not merely the last to have started.
    #[test]
    fn three_way_overlap_resolves_to_the_last_in_the_lane() {
        // A = 0..5s, B = 1..5s, C = 1..2s. B and C share a start, so
        // list order is what separates them.
        let (registry, mut timeline) =
            timeline_of(&[(0, 500), (100, 400), (100, 100)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // All three open at 0.2 — what A shows at 1s. At 1.5s their
        // own progress then separates them: A 0.3, B 0.325, C 0.7.
        let during =
            sample_at(&registry, &mut timeline, &mut world, cs(150));
        assert!(
            (during - 0.7).abs() < 1e-5,
            "the last clip in the lane should win, got {during}"
        );
    }

    /// A parked playhead must not rewrite finished clips every frame.
    /// The `time_range` guard is what prevents it.
    #[test]
    fn parked_playhead_stops_requeueing() {
        // A gap to park in — a single clip would leave the playhead
        // on its end, where it still counts as covered.
        let (registry, mut timeline) =
            timeline_of(&[(0, 100), (500, 100)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Crossing the first clip queues it.
        sample_at(&registry, &mut timeline, &mut world, cs(300));
        assert!(!timeline.queue_cache().is_empty());

        // Sitting still moves through nothing.
        sample_at(&registry, &mut timeline, &mut world, cs(300));
        assert!(
            timeline.queue_cache().is_empty(),
            "a parked playhead requeued a finished clip"
        );
    }

    /// Skipping *backwards* settles a lane on the value it held
    /// before any of its clips ran — `clips[0]`'s start, not the clip
    /// that finishes last.
    #[test]
    fn skipping_a_lane_backwards_holds_its_pre_lane_value() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<World>();

        // Somewhere to jump back to.
        let track0 = builder
            .act_builder(0u32, crate::path!(<f32>), |x| x + 1.0)
            .with_interp(linear)
            .play(cs(100))
            .compile();

        // 0..1s plus 0.5..3s over it, so the first stored clip is
        // not the one that finishes last.
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

        // Play it out so subject 1 is far from where it began.
        timeline.set_target_track(1);
        timeline.set_target_time(cs(300));
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);
        assert!(world.0[1] > 1.0, "got {}", world.0[1]);

        // Jump back: the skipped lane settles to 0.0. The clip that
        // finishes last would give 0.5, part-way up the short one.
        timeline.set_target_track(0);
        timeline.set_target_time(Duration::ZERO);
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);

        assert!(world.0[1].abs() < 1e-5, "got {}", world.0[1]);
    }

    /// Skipping forward settles the skipped lane on the clip that
    /// finishes last — with overlaps, not the last clip in the lane.
    #[test]
    fn track_skip_uses_the_clip_that_finishes_last() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<World>();

        // 0..10s stored first, 1..2s over it.
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

        // Somewhere to skip to, on another subject.
        let track1 = builder
            .act_builder(1u32, crate::path!(<f32>), |x| x + 1.0)
            .with_interp(linear)
            .play(cs(100))
            .compile();

        let tracks = TrackList::collect([track0, track1]).unwrap();
        let mut timeline = builder.compile(tracks);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        timeline.set_target_track(1);
        timeline.set_target_time(Duration::ZERO);
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);

        // The long clip finishes last: 0.0 -> 1.0. The short one
        // would leave 5.1.
        assert!(
            (world.0[0] - 1.0).abs() < 1e-5,
            "got {}",
            world.0[0]
        );
    }

    /// When two clips finish together, the later one in the lane owns
    /// what the lane rests on — both for a playhead sitting in the gap
    /// after them, and for the clip that opens on the far side of it.
    #[test]
    fn equal_ends_rest_on_the_later_clip() {
        // A = 0..5s and B = 1..5s finish together; C = 6..7s starts
        // after both, so nothing is covering it.
        let (registry, mut timeline) =
            timeline_of(&[(0, 500), (100, 400), (600, 100)]);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Parked in the gap. B is stored after A, so the lane rests
        // on B's end, 1.2. Resolving to A would read 1.0.
        let in_gap =
            sample_at(&registry, &mut timeline, &mut world, cs(550));
        assert!((in_gap - 1.2).abs() < 1e-5, "got {in_gap}");

        // C opens on that same resting value and runs 1.2 -> 2.2.
        // Opening it from A would read 1.5 here.
        let after_gap =
            sample_at(&registry, &mut timeline, &mut world, cs(650));
        assert!((after_gap - 1.7).abs() < 1e-5, "got {after_gap}");
    }

    /// Touching is not a gap: baking reads the earlier clip through
    /// its easing rather than short-circuiting to its end. Only an
    /// ease that never reaches 1.0 can tell the two apart.
    #[test]
    fn a_touching_clip_opens_on_the_eased_last_frame() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<World>();

        // Not normalised: tops out at 0.5, so the clip leaves the
        // screen half way through its range.
        let first = builder
            .act_builder(0u32, crate::path!(<f32>), |x| x + 1.0)
            .with_interp(linear)
            .with_ease(|t| t * 0.5)
            .play(cs(100));
        // Starts exactly where the first ends.
        let second = delay(
            cs(100),
            builder
                .act_builder(0u32, crate::path!(<f32>), |x| x + 1.0)
                .with_interp(linear)
                .play(cs(100)),
        );

        let track = [first, second].ord_all().compile();
        let mut timeline = builder.compile(track);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // 0.5, where the first clip actually left off — not 1.0, the
        // end value it never displayed.
        let handover =
            sample_at(&registry, &mut timeline, &mut world, cs(100));
        assert!((handover - 0.5).abs() < 1e-5, "got {handover}");
    }

    /// Two clips finishing at the same instant: the one later in the
    /// lane is what is on screen there, so it owns the resting value
    /// the transition path settles to.
    #[test]
    fn track_skip_breaks_end_ties_by_position() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<World>();

        // Both end at 5s: 0..5s stored first, 1..5s stored second.
        let first = builder
            .act_builder(0u32, crate::path!(<f32>), |x| x + 1.0)
            .with_interp(linear)
            .play(cs(500));
        let second = delay(
            cs(100),
            builder
                .act_builder(0u32, crate::path!(<f32>), |x| x + 5.0)
                .with_interp(linear)
                .play(cs(400)),
        );
        let track0 = [first, second].ord_all().compile();

        // Somewhere to skip to, driving a different subject.
        let track1 = builder
            .act_builder(1u32, crate::path!(<f32>), |x| x + 1.0)
            .with_interp(linear)
            .play(cs(100))
            .compile();

        let tracks = TrackList::collect([track0, track1]).unwrap();
        let mut timeline = builder.compile(tracks);

        let mut world = World::new();
        timeline.bake_actions(&registry, &world);

        // Skip forward, so track 0's lane settles to its resting
        // value.
        timeline.set_target_track(1);
        timeline.set_target_time(Duration::ZERO);
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);

        // `second` opens on what `first` shows at 1s (0.2) and runs
        // to 5.2. Breaking the tie the other way would leave 1.0.
        assert!(
            (world.0[0] - 5.2).abs() < 1e-5,
            "got {}",
            world.0[0]
        );
    }
}
