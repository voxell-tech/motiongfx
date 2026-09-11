use core::cmp::Ordering;
use core::marker::PhantomData;
use core::time::Duration;

use alloc::boxed::Box;
use field_path::field_accessor::FieldAccessor;
use hashbrown::{DefaultHashBuilder, HashMap};
use indexmap::IndexMap;
use motiongfx_interp::interpolation::Interpolation;

use crate::ThreadSafe;
use crate::action::{
    Action, ActionBuilder, ActionId, ActionKey, ActionTable,
    InterpActionBuilder, SampleMode,
};
use crate::pipeline::bake::{BakeClipCtx, BakeScratch};
use crate::pipeline::sample::SampleCtx;
use crate::pipeline::{PipelineKey, Range};
use crate::registry::Registry;
use crate::subject::SubjectId;
use crate::track::{Track, TrackList};
use crate::world::SubjectSource;

pub struct Timeline<W> {
    action_table: ActionTable,
    /// Track length is guaranteed to be at least 1 by construction.
    /// See [`TimelineBuilder::compile()`].
    tracks: Box<[Track]>,
    /// Fields to sample this frame, deduped per field and ordered so
    /// overlapping writes to aliasing memory resolve deterministically.
    queue: IndexMap<ActionKey, QueuedSample, DefaultHashBuilder>,
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

/// One field's resolved sample for the current frame.
#[derive(Debug, Clone, Copy)]
struct QueuedSample {
    id: ActionId,
    mode: SampleMode,
    /// The instant this sample represents.
    ///
    /// The clip's start for [`SampleMode::Start`] or end for
    /// [`SampleMode::End`], the playhead itself for
    /// [`SampleMode::Interp`].
    keyframe: Duration,
}

impl<W: 'static> Timeline<W> {
    /// Bakes every clip's [`Segment`](crate::action::Segment). Clips
    /// are visited in start-time order over a per-track working copy
    /// of each source, so an action composes on top of every earlier
    /// one.
    pub fn bake_actions(
        &mut self,
        registry: &Registry,
        subject_world: &W,
    ) {
        for track in self.tracks.iter() {
            let mut scratch = BakeScratch::default();

            for clip in track.bake_clips() {
                let Some(key) =
                    self.action_table.key(&clip.id).copied()
                else {
                    continue;
                };
                let pkey = PipelineKey::from_action_key::<W>(key);
                let ok = registry.pipeline.bake_clip(
                    &pkey,
                    BakeClipCtx {
                        world: subject_world,
                        subject: *key.subject_id(),
                        field: *key.field(),
                        action_id: clip.id,
                        scratch: &mut scratch,
                        action_table: &mut self.action_table,
                        accessor_registry: &registry.accessor,
                    },
                );
                debug_assert!(
                    ok,
                    "pipeline not found for key {pkey:?}"
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

        self.queue.clear();
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

                    let entry = match sample_mode {
                        SampleMode::Start => {
                            clips.first().map(|c| (c, c.start))
                        }
                        SampleMode::End => {
                            clips.last().map(|c| (c, c.end()))
                        }
                        SampleMode::Interp(_) => unreachable!(),
                    };
                    let Some((clip, keyframe)) = entry else {
                        continue;
                    };

                    self.queue.insert(
                        *key,
                        QueuedSample {
                            id: clip.id,
                            mode: sample_mode,
                            keyframe,
                        },
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

            let (Some(first), Some(last)) =
                (clips.first(), clips.last())
            else {
                continue;
            };
            let clips_range = Range {
                start: first.start,
                end: last.end(),
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

                    self.queue.insert(
                        *key,
                        QueuedSample {
                            id: clip.id,
                            mode: SampleMode::Interp(
                                clip.progress(self.target_time),
                            ),
                            keyframe: self.target_time,
                        },
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
                    let (sample_mode, keyframe) = if index == 0 {
                        (SampleMode::Start, clip.start)
                    } else {
                        (SampleMode::End, clip.end())
                    };

                    self.queue.insert(
                        *key,
                        QueuedSample {
                            id: clip.id,
                            mode: sample_mode,
                            keyframe,
                        },
                    );
                }
            }
        }

        // Farthest keyframe first, closest last, so overlapping
        // writes to aliasing memory land on the value nearest the
        // playhead every frame. `Interp` sits exactly on the playhead
        // (distance zero), so it always sorts last on its own.
        let target_time = self.target_time;
        self.queue.sort_unstable_by(|_, a, _, b| {
            target_time
                .abs_diff(b.keyframe)
                .cmp(&target_time.abs_diff(a.keyframe))
        });

        self.curr_time = self.target_time;
    }

    /// Writes every sample marked by [`Self::queue_actions`] into the
    /// world.
    pub fn sample_queued_actions(
        &self,
        registry: &Registry,
        subject_world: &mut W,
    ) {
        for (key, queued) in &self.queue {
            let pkey = PipelineKey::from_action_key::<W>(*key);
            let ok = registry.pipeline.sample(
                &pkey,
                SampleCtx {
                    world: subject_world,
                    action_table: &self.action_table,
                    accessor_registry: &registry.accessor,
                    samples: &[(queued.id, queued.mode)],
                },
            );
            debug_assert!(ok, "pipeline not found for key {pkey:?}");
        }
    }
}

// Getter methods.
impl<W> Timeline<W> {
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

    /// Every clip dropped across the tracks to resolve overlapping
    /// actions on aliasing field paths.
    #[cfg(feature = "diagnostics")]
    pub fn conflicts(
        &self,
    ) -> impl Iterator<Item = &crate::track::FieldConflict> {
        self.tracks.iter().flat_map(Track::conflicts)
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
        S: Clone + ThreadSafe,
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
        S: Clone + ThreadSafe,
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
        S: Clone + ThreadSafe,
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
        // `pipeline_counts` is builder-only; baking resolves a
        // pipeline per clip.
        Timeline {
            action_table: self.action_table,
            tracks: tracks.into().into_boxed_slice(),
            queue: IndexMap::default(),
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
mod tests {}
