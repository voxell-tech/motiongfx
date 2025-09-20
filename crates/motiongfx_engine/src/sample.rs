use core::cmp::Ordering;

use bevy::asset::AsAssetId;
use bevy::ecs::component::Mutable;
use bevy::prelude::*;

use crate::action::{ActionTarget, Ease, Interp};
use crate::bake::Segment;
use crate::field::{FieldRegistry, UntypedField};
use crate::prelude::Interpolation;
use crate::timeline_v2::{Timeline, TimelineSet};
use crate::ThreadSafe;

pub struct SamplePlugin;

impl Plugin for SamplePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            mark_actions_for_sampling.in_set(TimelineSet::Mark),
        );
    }
}

/// Mark tracks that overlaps with the current and target time
/// from the [`SequenceController`].
fn mark_actions_for_sampling(
    mut commands: Commands,
    mut q_timelines: Query<&mut Timeline, Changed<Timeline>>,
) {
    for mut timeline in q_timelines.iter_mut() {
        let timeline = timeline.bypass_change_detection();

        // Current time will change if the track index changes.
        let mut curr_time = timeline.curr_time();

        // Handle index changes.
        if timeline.target_index() != timeline.curr_index() {
            let (sample_mode, track_range) =
                if timeline.target_index() > timeline.curr_index() {
                    // From the start.
                    curr_time = 0.0;
                    (
                        SampleMode::End,
                        timeline.curr_index()
                            ..timeline.target_index(),
                    )
                } else {
                    // From the end.
                    curr_time = timeline.target_track().duration();
                    (
                        SampleMode::Start,
                        (timeline.target_index() + 1)
                            ..(timeline.curr_index() + 1),
                    )
                };

            for i in track_range {
                let track = &timeline.tracks()[i];

                for (_, spans) in track.iter_sequences() {
                    let span = match sample_mode {
                        SampleMode::Start => spans.first(),
                        SampleMode::End => spans.last(),
                        SampleMode::Interp(_) => unreachable!(),
                    };

                    if let Some(span) = span {
                        commands
                            .entity(span.action_id())
                            .insert(sample_mode);
                    }
                }
            }

            timeline.sync_curr_track();
        }

        let track = timeline.curr_track();

        let time_range = Range {
            start: curr_time.min(timeline.target_time()),
            end: curr_time.max(timeline.target_time()),
        };

        for (_, spans) in track.iter_sequences() {
            let start = spans.first().unwrap().start_time();
            let end = spans.last().unwrap().end_time();
            let seq_range = Range { start, end };

            if !time_range.overlap(&seq_range) {
                continue;
            }

            // If the returned `index` is `Ok`, the target time is
            // within `span[index]`.
            //
            // If the returned `index` is `Err`, the target time is
            // before the sequence if `index == 0`, otherwise,
            // after `span[index - 1]`
            let index = spans.binary_search_by(|span| {
                if timeline.target_time() < span.start_time() {
                    Ordering::Greater
                } else if timeline.target_time() > span.end_time() {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            });

            match index {
                // `target_time` is within a segment.
                Ok(index) => {
                    let span = &spans[index];

                    let percent = (timeline.target_time()
                        - span.start_time())
                        / (span.end_time() - span.start_time());

                    commands
                        .entity(span.action_id())
                        .insert(SampleMode::Interp(percent));
                }
                // `target_time` is out of bounds.
                Err(index) => {
                    let span = &spans[index.saturating_sub(1)];

                    let span_range = Range {
                        start: span.start_time(),
                        end: span.end_time(),
                    };
                    // Skip if the the animation range does not
                    // overlap with the span range.
                    if time_range.overlap(&span_range) == false {
                        continue;
                    }

                    if index == 0 {
                        // Target time is before the entire sequence.
                        commands
                            .entity(span.action_id())
                            .insert(SampleMode::Start);
                    } else {
                        // Target time is after `index - 1`.
                        // Indexing taken care by the saturating sub
                        // above.
                        commands
                            .entity(span.action_id())
                            .insert(SampleMode::End);
                    }
                }
            }
        }

        timeline.sync_curr_time();
    }
}

/// Query type alias for sampling segments.
type SampleQuery<'a, Target> = Query<
    'a,
    'a,
    (
        &'a Segment<Target>,
        Option<&'a Interp<Target>>,
        Option<&'a Ease>,
        &'a SampleMode,
        &'a ActionTarget,
        &'a UntypedField,
        Entity,
    ),
>;

fn sample_component_segments<Source, Target>(
    mut commands: Commands,
    mut q_components: Query<&mut Source>,
    q_segments: SampleQuery<Target>,
    field_registry: Res<FieldRegistry>,
) -> Result
where
    Source: Component<Mutability = Mutable>,
    Target: Interpolation + Clone + ThreadSafe,
{
    for (segment, interp, ease, sample_mode, target, field, entity) in
        q_segments.iter()
    {
        commands.entity(entity).remove::<SampleMode>();

        let Ok(mut source) = q_components.get_mut(target.entity())
        else {
            continue;
        };

        let value = match sample_mode {
            SampleMode::Start => segment.start().clone(),
            SampleMode::End => segment.end().clone(),
            SampleMode::Interp(mut percent) => {
                if let Some(ease) = ease {
                    percent = ease(percent);
                }

                if let Some(interp) = interp {
                    interp(segment.start(), segment.end(), percent)
                } else {
                    Target::interp(
                        segment.start(),
                        segment.end(),
                        percent,
                    )
                }
            }
        };

        let accessor = field_registry
            .get_accessor(*field)
            .ok_or(format!("No accessor for {field:?}"))?;

        *accessor.get_mut(source.as_mut()) = value;
    }

    Ok(())
}

#[cfg(feature = "asset")]
fn sample_asset_segments<Source, Target>(
    mut commands: Commands,
    q_components: Query<&Source>,
    mut assets: ResMut<Assets<Source::Asset>>,
    q_segments: SampleQuery<Target>,
    field_registry: Res<FieldRegistry>,
) -> Result
where
    Source: AsAssetId<Mutability = Mutable>,
    Target: Interpolation + Clone + ThreadSafe,
{
    for (segment, interp, ease, sample_mode, target, field, entity) in
        q_segments.iter()
    {
        commands.entity(entity).remove::<SampleMode>();

        let Some(source) = q_components
            .get(target.entity())
            .ok()
            .and_then(|s| assets.get_mut(s.as_asset_id()))
        else {
            continue;
        };

        let value = match sample_mode {
            SampleMode::Start => segment.start().clone(),
            SampleMode::End => segment.end().clone(),
            SampleMode::Interp(mut percent) => {
                if let Some(ease) = ease {
                    percent = ease(percent);
                }

                if let Some(interp) = interp {
                    interp(segment.start(), segment.end(), percent)
                } else {
                    Target::interp(
                        segment.start(),
                        segment.end(),
                        percent,
                    )
                }
            }
        };

        let accessor = field_registry
            .get_accessor(*field)
            .ok_or(format!("No accessor for {field:?}"))?;

        *accessor.get_mut(source) = value;
    }

    Ok(())
}

/// Determines how a [`Segment`] should be sampled.
#[derive(Component, Debug, Clone, Copy)]
#[component(storage = "SparseSet", immutable)]
enum SampleMode {
    Start,
    End,
    Interp(f32),
}

#[derive(Default, Debug, PartialEq, Clone, Copy)]
struct Range {
    start: f32,
    end: f32,
}

impl Range {
    /// Calculate if 2 [`Range`]s overlap.
    pub fn overlap(&self, other: &Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}
