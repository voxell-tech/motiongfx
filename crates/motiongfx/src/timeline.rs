use core::cmp::Ordering;
use core::marker::PhantomData;
use core::time::Duration;

use alloc::boxed::Box;
use alloc::vec::Vec;
use field_path::field_accessor::FieldAccessor;
use hashbrown::HashMap;

use crate::ThreadSafe;
use crate::action::{
    Action, ActionBuilder, ActionId, ActionKey, ActionTable,
    InterpActionBuilder, SampleMode,
};
use crate::interpolation::Interpolation;
use crate::pipeline::{BakeCtx, PipelineKey, Range, SampleCtx};
use crate::registry::Registry;
use crate::subject::SubjectId;
use crate::time::{self, IntoDuration};
use crate::track::Track;
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

                    self.queue_cache.cache(
                        *key,
                        clip.id,
                        SampleMode::Interp(
                            clip.progress(self.target_time),
                        ),
                    );
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
                    if !time_range.overlap(&clip_range) {
                        continue;
                    }

                    // Target time before the sequence -> Start,
                    // otherwise it is past `index - 1` -> End (the
                    // saturating sub above handles the indexing).
                    let sample_mode = if index == 0 {
                        SampleMode::Start
                    } else {
                        SampleMode::End
                    };

                    self.queue_cache.cache(
                        *key,
                        clip.id,
                        sample_mode,
                    );
                }
            }
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
        target_time: impl IntoDuration,
    ) -> &mut Self {
        let duration = self.tracks[self.target_index].duration();

        self.target_time = target_time.into_duration().min(duration);
        self
    }

    /// Steps the target time by `delta` seconds, saturating at both
    /// ends of the target track.
    ///
    /// Prefer this over adding to [`Self::target_time`]: a [`Duration`]
    /// cannot go negative, so `delta < 0.0` needs saturating
    /// arithmetic.
    pub fn advance_secs(&mut self, delta: f32) -> &mut Self {
        let target_time = time::offset_secs(self.target_time, delta);

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
    tracks: Vec<Track>,
    _marker: PhantomData<fn() -> W>,
}

impl<'a, W: 'static> TimelineBuilder<'a, W> {
    /// Creates an empty timeline builder.
    pub fn new(registry: &'a mut Registry) -> Self {
        Self {
            registry,
            action_table: ActionTable::new(),
            pipeline_counts: HashMap::new(),
            tracks: Vec::new(),
            _marker: PhantomData,
        }
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
    /// Use [`Self::try_compile`] to explicitly handle the case where
    /// the track may be empty.
    pub fn compile(self) -> Timeline<W> {
        // TODO(nixon): What happens when track is empty?
        debug_assert!(
            !self.tracks.is_empty(),
            "Track cannot be empty!"
        );

        Timeline {
            action_table: self.action_table,
            pipeline_counts: self
                .pipeline_counts
                .into_iter()
                .collect(),
            tracks: self.tracks.into_boxed_slice(),
            queue_cache: QueueCache::new(),
            sample_queue: HashMap::new(),
            curr_time: Duration::ZERO,
            target_time: Duration::ZERO,
            curr_index: 0,
            target_index: 0,
            _marker: PhantomData,
        }
    }

    /// Similar to [`Self::compile`] but return `None` instead of
    /// panicking.
    pub fn try_compile(self) -> Option<Timeline<W>> {
        (!self.tracks.is_empty()).then(|| self.compile())
    }
}

// TODO: Write some unit tests.
#[cfg(test)]
mod tests {}
