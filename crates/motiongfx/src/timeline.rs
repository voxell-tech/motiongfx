use core::cmp::Ordering;
use core::marker::PhantomData;

use alloc::boxed::Box;
use alloc::vec::Vec;
use bevy_platform::collections::HashMap;
use field_path::field_accessor::FieldAccessor;

use crate::ThreadSafe;
use crate::action::{
    Action, ActionBuilder, ActionId, ActionKey, ActionWorld,
    InterpActionBuilder, SampleMode,
};
use crate::pipeline::{
    BakeCtx, PipelineKey, Range, SampleCtx, SubjectSource,
};
use crate::registry::Registry;
use crate::subject::SubjectId;
use crate::track::Track;

pub struct Timeline<W> {
    action_world: ActionWorld,
    pipeline_counts: Box<[(PipelineKey, u32)]>,
    /// Track length is guaranteed to be at least 1 by construction.
    /// See [`TimelineBuilder::compile()`].
    tracks: Box<[Track]>,
    /// Cached actions that are queued to be sampled.
    ///
    /// This cache will be cleared everytime [`Timeline::queue_actions`]
    /// is called.
    queue_cache: QueueCache,
    /// The current time of the current track.
    curr_time: f32,
    /// The target time of the target track.
    target_time: f32,
    /// The index of the current track.
    curr_index: usize,
    /// The index of the target track.
    target_index: usize,
    _marker: PhantomData<fn() -> W>,
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
                        action_world: &mut self.action_world,
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
                curr_time = 0.0;
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
                for (key, span) in self.tracks[i].sequences_spans() {
                    if span.len == 0 {
                        continue;
                    }

                    let clips = self.tracks[i].clips(*span);

                    // SAFETY: `clips` is not empty.
                    let clip = match sample_mode {
                        SampleMode::Start => clips.first().unwrap(),
                        SampleMode::End => clips.last().unwrap(),
                        SampleMode::Interp(_) => unreachable!(),
                    };

                    self.queue_cache.cache(
                        *key,
                        clip.id,
                        &mut self.action_world,
                    );

                    self.action_world
                        .edit_action(clip.id)
                        .mark(sample_mode);
                }
            }

            self.curr_index = self.target_index;
        }

        let time_range = Range {
            start: curr_time.min(self.target_time()),
            end: curr_time.max(self.target_time()),
        };

        for (key, span) in
            self.tracks[self.curr_index].sequences_spans()
        {
            if span.len == 0 {
                continue;
            }

            let clips = self.tracks[self.curr_index].clips(*span);

            // SAFETY: `clips` is not empty.
            let clips_range = Range {
                start: clips.first().unwrap().start,
                end: clips.last().unwrap().end(),
            };

            if !time_range.overlap(&clips_range) {
                continue;
            }

            // If the returned `index` is `Ok`, the target time is
            // within `span[index]`.
            //
            // If the returned `index` is `Err`, the target time is
            // before the sequence if `index == 0`, otherwise,
            // after `span[index - 1]`
            let index = clips.binary_search_by(|clip| {
                if self.target_time() < clip.start {
                    Ordering::Greater
                } else if self.target_time() > clip.end() {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            });

            match index {
                // `target_time` is within a segment.
                Ok(index) => {
                    let clip = &clips[index];

                    let t = (self.target_time - clip.start)
                        / (clip.end() - clip.start);

                    self.queue_cache.cache(
                        *key,
                        clip.id,
                        &mut self.action_world,
                    );

                    self.action_world
                        .edit_action(clip.id)
                        .mark(SampleMode::Interp(t));
                }
                // `target_time` is out of bounds.
                Err(index) => {
                    let clip = &clips[index.saturating_sub(1)];

                    let clip_range = Range {
                        start: clip.start,
                        end: clip.end(),
                    };
                    // Skip if the the animation range does not
                    // overlap with the span range.
                    if time_range.overlap(&clip_range) == false {
                        continue;
                    }

                    self.queue_cache.cache(
                        *key,
                        clip.id,
                        &mut self.action_world,
                    );
                    let mut action_cmd =
                        self.action_world.edit_action(clip.id);

                    if index == 0 {
                        // Target time is before the entire sequence.
                        action_cmd.mark(SampleMode::Start);
                    } else {
                        // Target time is after `index - 1`.
                        // Indexing taken care by the saturating sub
                        // above.
                        action_cmd.mark(SampleMode::End);
                    }
                }
            }
        }

        self.curr_time = self.target_time;
    }

    pub fn sample_queued_actions(
        &self,
        registry: &Registry,
        subject_world: &mut W,
    ) {
        for key in self.pipeline_counts.iter().map(|(key, _)| key) {
            let ok = registry.pipeline.sample(
                key,
                SampleCtx {
                    world: subject_world,
                    action_world: &self.action_world,
                    accessor_registry: &registry.accessor,
                },
            );
            debug_assert!(ok, "pipeline not found for key {key:?}");
        }
    }

    fn reset_queues(&mut self) {
        self.queue_cache.clear();
        self.action_world.clear_all_marks();
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
    pub fn set_target_time(&mut self, target_time: f32) -> &mut Self {
        let duration = self.tracks[self.target_index].duration();

        self.target_time = target_time.clamp(0.0, duration);
        self
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
    cache: HashMap<ActionKey, ActionId>,
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
    ) -> impl Iterator<Item = (&ActionKey, &ActionId)> {
        self.cache.iter()
    }

    pub fn iter_keys(&self) -> impl Iterator<Item = &ActionKey> {
        self.cache.keys()
    }

    pub fn iter_ids(&self) -> impl Iterator<Item = &ActionId> {
        self.cache.values()
    }

    /// Clear all the cached contents.
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Cache an [`ActionKey`] while deduplicating the old cache if
    /// it exists.
    pub fn cache(
        &mut self,
        key: ActionKey,
        id: ActionId,
        action_world: &mut ActionWorld,
    ) {
        if let Some(prev_id) = self.cache.insert(key, id) {
            action_world.edit_action(prev_id).clear_mark();
        }
    }
}

impl Default for QueueCache {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TimelineBuilder<'a, W> {
    registry: &'a mut Registry,
    action_world: ActionWorld,
    pipeline_counts: HashMap<PipelineKey, u32>,
    tracks: Vec<Track>,
    _marker: PhantomData<fn() -> W>,
}

impl<'a, W: 'static> TimelineBuilder<'a, W> {
    /// Creates an empty timeline builder.
    pub fn new(registry: &'a mut Registry) -> Self {
        Self {
            registry,
            action_world: ActionWorld::new(),
            pipeline_counts: HashMap::new(),
            tracks: Vec::new(),
            _marker: PhantomData,
        }
    }

    /// Add an [`Action`] without interpolation.
    pub fn act<I, S, T>(
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

        self.action_world.add(target, field, action)
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
        self.act(target, field_acc, action).with_interp(|a, b, t| {
            if t < 1.0 { a.clone() } else { b.clone() }
        })
    }

    /// Remove an [`Action`].
    pub fn unact(&mut self, id: ActionId) -> bool {
        if let Some(key) = self.action_world.remove(id) {
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

    /// Add [`Track`]\(s\) to the timeline.
    pub fn add_tracks(
        &mut self,
        tracks: impl IntoIterator<Item = Track>,
    ) {
        self.tracks.extend(tracks);
    }

    /// Compile into a [`Timeline`].
    ///
    /// ## Panic
    ///
    /// Panics if the track is empty.
    pub fn compile(self) -> Timeline<W> {
        // TODO(nixon): What happens when track is empty?
        debug_assert!(
            !self.tracks.is_empty(),
            "Track cannot be empty!"
        );

        Timeline {
            action_world: self.action_world,
            pipeline_counts: self
                .pipeline_counts
                .into_iter()
                .collect(),
            tracks: self.tracks.into_boxed_slice(),
            queue_cache: QueueCache::new(),
            curr_time: 0.0,
            target_time: 0.0,
            curr_index: 0,
            target_index: 0,
            _marker: PhantomData,
        }
    }

    /// Similar to [`Self::compile()`] but return `None` instead of
    /// panicking.
    pub fn try_compile(self) -> Option<Timeline<W>> {
        (!self.tracks.is_empty()).then(|| self.compile())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path;
    use crate::pipeline::SubjectSource;

    // ── Shared test infrastructure ────────────────────────────────────────────

    /// A simple world backed by a Vec of f32 values.
    /// The subject id is `usize` (index into the vec).
    struct VecWorld(alloc::vec::Vec<f32>);

    impl SubjectSource<usize, f32> for VecWorld {
        fn get_source(&self, id: usize) -> Option<&f32> {
            self.0.get(id)
        }

        fn apply_source<R>(
            &mut self,
            id: usize,
            f: impl FnOnce(&mut f32) -> R,
        ) -> Option<R> {
            self.0.get_mut(id).map(f)
        }
    }

    fn linear_f32(a: &f32, b: &f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    // ── TimelineBuilder ───────────────────────────────────────────────────────

    #[test]
    fn timeline_builder_try_compile_returns_none_when_no_tracks() {
        let mut registry = Registry::new();
        let builder = registry.create_builder::<VecWorld>();
        assert!(builder.try_compile().is_none());
    }

    #[test]
    fn timeline_builder_registers_pipeline_on_act() {
        // Verify that calling `act` registers the pipeline such that the
        // subsequent bake-sample cycle completes without panicking.
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<VecWorld>();

        let frag = builder
            .act(0_usize, path!(<f32>), |x| x + 5.0)
            .with_interp(linear_f32)
            .play(1.0);
        builder.add_tracks(frag.compile());

        let mut timeline = builder.compile();
        let mut world = VecWorld(alloc::vec![0.0]);

        // If the pipeline was not registered, `bake_actions` would
        // debug_assert / panic. This verifies registration happened.
        timeline.bake_actions(&registry, &world);
        timeline.set_target_time(1.0);
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);

        assert!((world.0[0] - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn timeline_builder_act_step_produces_step_interpolation() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<VecWorld>();

        let frag =
            builder.act_step(0_usize, path!(<f32>), |x| x + 10.0).play(1.0);
        let track = frag.compile();
        builder.add_tracks(track);
        let mut timeline = builder.compile();

        let mut world = VecWorld(alloc::vec![0.0]);
        timeline.bake_actions(&registry, &world);

        // At t=0.5 step-interpolation should stay at 0.0 (start value).
        timeline.set_target_time(0.5);
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);

        // Step interp returns start until t == 1.0.
        assert_eq!(world.0[0], 0.0);

        // At t=1.0 step-interpolation should jump to the end value.
        timeline.set_target_time(1.0);
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);
        assert_eq!(world.0[0], 10.0);
    }

    // ── Timeline (core bake / sample cycle) ───────────────────────────────────

    #[test]
    fn timeline_bake_and_sample_at_midpoint() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<VecWorld>();

        let frag = builder
            .act(0_usize, path!(<f32>), |x| x + 10.0)
            .with_interp(linear_f32)
            .play(1.0);

        let track = frag.compile();
        builder.add_tracks(track);
        let mut timeline = builder.compile();

        let mut world = VecWorld(alloc::vec![0.0]);

        // Bake once before sampling.
        timeline.bake_actions(&registry, &world);

        timeline.set_target_time(0.5);
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);

        let expected = 5.0_f32;
        assert!(
            (world.0[0] - expected).abs() < f32::EPSILON,
            "At t=0.5 expected {expected}, got {}",
            world.0[0]
        );
    }

    #[test]
    fn timeline_bake_and_sample_at_end() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<VecWorld>();

        let frag = builder
            .act(0_usize, path!(<f32>), |x| x + 10.0)
            .with_interp(linear_f32)
            .play(1.0);

        builder.add_tracks(frag.compile());
        let mut timeline = builder.compile();
        let mut world = VecWorld(alloc::vec![0.0]);

        timeline.bake_actions(&registry, &world);

        timeline.set_target_time(1.0);
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);

        assert!(
            (world.0[0] - 10.0).abs() < f32::EPSILON,
            "At t=1.0 expected 10.0, got {}",
            world.0[0]
        );
    }

    #[test]
    fn timeline_bake_and_sample_at_start() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<VecWorld>();

        let frag = builder
            .act(0_usize, path!(<f32>), |x| x + 10.0)
            .with_interp(linear_f32)
            .play(1.0);

        builder.add_tracks(frag.compile());
        let mut timeline = builder.compile();
        let mut world = VecWorld(alloc::vec![0.0]);

        timeline.bake_actions(&registry, &world);

        // Sampling at t=0.0 should produce the start value.
        timeline.set_target_time(0.0);
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);

        assert_eq!(world.0[0], 0.0);
    }

    // ── Timeline setters ─────────────────────────────────────────────────────

    #[test]
    fn set_target_time_clamps_below_zero() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<VecWorld>();
        let frag = builder
            .act(0_usize, path!(<f32>), |x| x + 1.0)
            .with_interp(linear_f32)
            .play(2.0);
        builder.add_tracks(frag.compile());
        let mut timeline = builder.compile();

        timeline.set_target_time(-5.0);
        assert_eq!(timeline.target_time(), 0.0);
    }

    #[test]
    fn set_target_time_clamps_above_duration() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<VecWorld>();
        let frag = builder
            .act(0_usize, path!(<f32>), |x| x + 1.0)
            .with_interp(linear_f32)
            .play(2.0);
        builder.add_tracks(frag.compile());
        let mut timeline = builder.compile();

        timeline.set_target_time(999.0);
        assert_eq!(timeline.target_time(), 2.0);
    }

    #[test]
    fn set_target_track_clamps_above_last_index() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<VecWorld>();
        let frag = builder
            .act(0_usize, path!(<f32>), |x| x + 1.0)
            .with_interp(linear_f32)
            .play(1.0);
        builder.add_tracks(frag.compile());
        let mut timeline = builder.compile();

        timeline.set_target_track(999);
        assert_eq!(timeline.target_index(), 0); // only one track at index 0
    }

    // ── Timeline completion detection ─────────────────────────────────────────

    #[test]
    fn timeline_is_not_complete_at_start() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<VecWorld>();
        let frag = builder
            .act(0_usize, path!(<f32>), |x| x + 1.0)
            .with_interp(linear_f32)
            .play(1.0);
        builder.add_tracks(frag.compile());
        let mut timeline = builder.compile();

        assert!(!timeline.is_complete());
    }

    #[test]
    fn timeline_is_complete_at_end_of_last_track() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<VecWorld>();
        let frag = builder
            .act(0_usize, path!(<f32>), |x| x + 1.0)
            .with_interp(linear_f32)
            .play(1.0);
        builder.add_tracks(frag.compile());
        let mut timeline = builder.compile();
        let mut world = VecWorld(alloc::vec![0.0]);

        timeline.bake_actions(&registry, &world);
        timeline.set_target_time(1.0);
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);

        assert!(timeline.is_complete());
    }

    #[test]
    fn timeline_is_last_track_with_single_track() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<VecWorld>();
        let frag = builder
            .act(0_usize, path!(<f32>), |x| x + 1.0)
            .with_interp(linear_f32)
            .play(1.0);
        builder.add_tracks(frag.compile());
        let timeline = builder.compile();

        assert!(timeline.is_last_track());
    }

    // ── QueueCache ────────────────────────────────────────────────────────────

    #[test]
    fn queue_cache_starts_empty() {
        let cache = QueueCache::new();
        assert!(cache.is_empty());
    }

    #[test]
    fn queue_cache_clears_on_demand() {
        // Indirectly test clear via the timeline: after queue_actions the
        // cache is populated, but after a second call it should be refreshed.
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<VecWorld>();
        let frag = builder
            .act(0_usize, path!(<f32>), |x| x + 1.0)
            .with_interp(linear_f32)
            .play(1.0);
        builder.add_tracks(frag.compile());
        let mut timeline = builder.compile();
        let mut world = VecWorld(alloc::vec![0.0]);

        timeline.bake_actions(&registry, &world);
        timeline.set_target_time(0.5);
        timeline.queue_actions();
        // After queue, cache should be non-empty.
        assert!(!timeline.queue_cache().is_empty());

        // Calling queue again resets the cache before re-populating.
        timeline.queue_actions();
        assert!(!timeline.queue_cache().is_empty());
    }

    // ── Multi-subject animation ───────────────────────────────────────────────

    #[test]
    fn timeline_animates_multiple_subjects_independently() {
        let mut registry = Registry::new();
        let mut builder = registry.create_builder::<VecWorld>();

        // Both subjects animated concurrently in a single track using ord_all.
        use crate::track::TrackOrdering;
        let frag0 = builder
            .act(0_usize, path!(<f32>), |x| x + 10.0)
            .with_interp(linear_f32)
            .play(1.0);
        let frag1 = builder
            .act(1_usize, path!(<f32>), |x| x + 20.0)
            .with_interp(linear_f32)
            .play(1.0);

        let track = [frag0, frag1].ord_all().compile();

        builder.add_tracks(track);
        let mut timeline = builder.compile();
        let mut world = VecWorld(alloc::vec![0.0, 0.0]);

        timeline.bake_actions(&registry, &world);
        timeline.set_target_time(1.0);
        timeline.queue_actions();
        timeline.sample_queued_actions(&registry, &mut world);

        assert!(
            (world.0[0] - 10.0).abs() < f32::EPSILON,
            "Subject 0 expected 10.0, got {}",
            world.0[0]
        );
        assert!(
            (world.0[1] - 20.0).abs() < f32::EPSILON,
            "Subject 1 expected 20.0, got {}",
            world.0[1]
        );
    }
}
