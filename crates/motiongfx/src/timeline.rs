use core::cmp::Ordering;

use alloc::boxed::Box;
use alloc::vec::Vec;
use bevy_ecs::prelude::*;
use bevy_platform::collections::HashMap;

use crate::accessor::FieldAccessorRegistry;
use crate::action::*;
use crate::field::UntypedField;
use crate::interpolation::{step, Interpolation};
use crate::pipeline::*;
use crate::track::*;
use crate::ThreadSafe;

#[derive(Component)]
pub struct Timeline {
    action_world: ActionWorld,
    pipeline_counts: Box<[(PipelineKey, u32)]>,
    tracks: Box<[Track]>,
    /// Cached actions that are queued to be sampled.
    ///
    /// This cache will be cleared everytime [`Timeline::queue_actions`]
    /// is called.
    queue_cahce: QueueCache,
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

impl Timeline {
    pub fn bake_actions(
        &mut self,
        pipeline_registry: &PipelineRegistry,
        target_world: &World,
        accessor_registry: &FieldAccessorRegistry,
    ) {
        for key in self.pipeline_counts.iter().map(|(key, _)| key) {
            let Some(pipeline) = pipeline_registry.get(key) else {
                continue;
            };

            for track in self.tracks.iter() {
                pipeline.bake(BakeCtx {
                    track,
                    action_world: &mut self.action_world,
                    target_world,
                    accessor_registry,
                })
            }
        }
    }

    pub fn queue_actions(&mut self) {
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

                    self.queue_cahce.cache(
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

                    self.queue_cahce.cache(
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

                    self.queue_cahce.cache(
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
        pipeline_registry: &PipelineRegistry,
        target_world: &mut World,
        accessor_registry: &FieldAccessorRegistry,
    ) {
        for key in self.pipeline_counts.iter().map(|(key, _)| key) {
            let Some(pipeline) = pipeline_registry.get(key) else {
                continue;
            };

            pipeline.sample(SampleCtx {
                action_world: &self.action_world,
                target_world,
                accessor_registry,
            });
        }
    }

    fn reset_queues(&mut self) {
        self.queue_cahce.clear();
        self.action_world.clear_all_marks();
    }
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
    /// # Panics
    ///
    /// Panics if out of bounds in `debug_assertions`.
    pub fn set_target_time(&mut self, target_time: f32) -> &mut Self {
        let duration = self.tracks[self.target_index].duration();

        debug_assert!(target_time < 0.0 || target_time > duration);

        self.target_time = target_time.clamp(0.0, duration);
        self
    }

    /// Set the target track index, clamping the value within
    /// \[0..=track_count - 1\].
    ///
    /// # Panics
    ///
    /// Panics if out of bounds in `debug_assertions`.
    pub fn set_target_track(
        &mut self,
        target_index: usize,
    ) -> &mut Self {
        let max_index = self.last_track_index();

        debug_assert!(target_index > max_index);

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
#[derive(Default)]
pub struct QueueCache {
    cache: HashMap<ActionKey, ActionId>,
}

impl QueueCache {
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

#[derive(Default)]
pub struct TimelineBuilder {
    action_world: ActionWorld,
    pipeline_counts: HashMap<PipelineKey, u32>,
    tracks: Vec<Track>,
}

impl TimelineBuilder {
    /// Add an [`Action`] without interpolation.
    pub fn act<T>(
        &mut self,
        action: impl Action<T>,
        target: impl Into<ActionTarget>,
        field: impl Into<UntypedField>,
    ) -> ActionBuilder<'_, T>
    where
        T: ThreadSafe,
    {
        let field = field.into();
        let key = PipelineKey::from_field(field);

        match self.pipeline_counts.get_mut(&key) {
            Some(count) => *count += 1,
            None => {
                self.pipeline_counts.insert(key, 1);
            }
        }

        self.action_world.add(action, target, field)
    }

    /// Add an [`Action`] with interpolation using
    /// [`Interpolation::interp`].
    pub fn act_interp<T>(
        &mut self,
        action: impl Action<T>,
        target: impl Into<ActionTarget>,
        field: impl Into<UntypedField>,
    ) -> InterpolatedActionBuilder<'_, T>
    where
        T: Interpolation + ThreadSafe,
    {
        self.act(action, target, field).with_interp(T::interp)
    }

    /// Add an [`Action`] with interpolation using [`step`].
    pub fn act_step<T>(
        &mut self,
        action: impl Action<T>,
        target: impl Into<ActionTarget>,
        field: impl Into<UntypedField>,
    ) -> InterpolatedActionBuilder<'_, T>
    where
        T: Clone + ThreadSafe,
    {
        self.act(action, target, field)
            .with_interp(|a, b, t| step(a.clone(), b.clone(), t))
    }

    /// Remove an [`Action`].
    pub fn unact(&mut self, id: ActionId) -> bool {
        if let Some(ActionKey { field, .. }) =
            self.action_world.remove(id)
        {
            let key = PipelineKey::from_field(field);

            let count = self
                .pipeline_counts
                .get_mut(&key)
                .unwrap_or_else(|| {
                    panic!(
                        "Field counts not registered for {field:?}!"
                    )
                });

            *count -= 1;
            if *count == 0 {
                self.pipeline_counts.remove(&key);
            }

            return true;
        }

        false
    }

    /// Add [`Track`]\(s\) to the timeline.
    pub fn add_tracks(
        &mut self,
        tracks: impl Iterator<Item = Track>,
    ) {
        self.tracks.extend(tracks);
    }

    pub fn compile(self) -> Timeline {
        Timeline {
            action_world: self.action_world,
            pipeline_counts: self
                .pipeline_counts
                .into_iter()
                .collect(),
            tracks: self.tracks.into_boxed_slice(),
            queue_cahce: QueueCache::default(),
            is_playing: false,
            time_scale: 1.0,
            curr_time: 0.0,
            target_time: 0.0,
            curr_index: 0,
            target_index: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
}

// fn style_1() {
//     let mut b = TimelineBuilder::new();

//     let track_0 = [
//         t.track_fragment(..),
//         t.track_fragment(..),
//         t.track_fragment(..),
//     ]
//     .flow(1.0);

//     let track_1 = [
//         t.track_fragment(..),
//         t.track_fragment(..),
//         t.track_fragment(..),
//     ]
//     .all();

//     let track_2 = [
//         t.track_fragment(..),
//         t.track_fragment(..),
//         t.track_fragment(..),
//     ]
//     .chain();

//     b.add_tracks([track_0, track_1, track_2]);
//     let timeline = b.compile();
// }

// fn style_1() {
//     let mut b = TimelineBuilder::new();

//     let track = [
//         t.track_fragment(..),
//         t.track_fragment(..),
//         t.track_fragment(..),
//     ]
//     .flow(1.0);

//     b.chain(track).set_checkpoint();

//     let track = [
//         t.track_fragment(..),
//         t.track_fragment(..),
//         t.track_fragment(..),
//     ]
//     .all();

//     b.chain(track).set_checkpoint();

//     let track_2 = [
//         t.track_fragment(..),
//         t.track_fragment(..),
//         t.track_fragment(..),
//     ]
//     .chain();

//     b.chain(track);
//     let timeline = b.compile();
// }
