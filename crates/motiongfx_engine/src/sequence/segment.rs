use std::cmp::Ordering;

use bevy::asset::AsAssetId;
use bevy::ecs::component::Mutable;
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::{
    IntoObserverSystem, ObserverSystem, ScheduleSystem, SystemParam,
};
use bevy::prelude::*;

use crate::action::{Action, ActionTarget, Ease, Interp};
use crate::field::{Field, FieldAccessor, FieldMap};
use crate::prelude::{FieldHash, Interpolation};
use crate::{MotionGfxSet, ThreadSafe};

use super::track::Tracks;
use super::{Sequence, SequenceController};

pub(super) struct KeyframePlugin;

impl Plugin for KeyframePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            mark_actions_for_sampling
                .in_set(MotionGfxSet::MarkAction),
        );
    }
}

/// Mark tracks that overlaps with the current and target time
/// from the [`SequenceController`].
fn mark_actions_for_sampling(
    mut commands: Commands,
    q_sequences: Query<
        (&Sequence, &SequenceController, &Tracks),
        Changed<SequenceController>,
    >,
) {
    for (sequence, controller, tracks) in q_sequences.iter() {
        let animate_range = Range {
            begin: controller.curr_time().min(controller.target_time),
            end: controller.curr_time().max(controller.target_time),
        };

        for track in tracks.values() {
            let track_range = Range {
                begin: track.start_time(),
                end: track.end_time(),
            };

            if animate_range.overlap(&track_range) == false {
                continue;
            }

            let span_ids = track.span_ids();

            let index = span_ids.binary_search_by(|span_id| {
                let span = sequence.spans[*span_id];

                if controller.target_time < span.start_time() {
                    Ordering::Greater
                } else if controller.target_time > span.end_time() {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            });

            match index {
                // `target_time` is within a segment.
                Ok(index) => {
                    let span_id = span_ids[index];
                    let span = &sequence.spans[span_id];

                    let percent = (controller.target_time
                        - span.start_time())
                        / (span.end_time() - span.start_time());

                    commands
                        .entity(span.action_id())
                        .insert(SampleType::Interp(percent));
                }
                // `target_time` is out of bounds.
                Err(index) => {
                    let span = &sequence.spans
                        [span_ids[index.saturating_sub(1)]];

                    let span_range = Range {
                        begin: span.start_time(),
                        end: span.end_time(),
                    };
                    // Skip if the the animation range does not
                    // overlap with the span range.
                    if animate_range.overlap(&span_range) == false {
                        continue;
                    }

                    if index == 0 {
                        commands
                            .entity(span.action_id())
                            .insert(SampleType::Start);
                    } else {
                        commands
                            .entity(span.action_id())
                            .insert(SampleType::End);
                    }
                }
            }
        }
    }
}

/// Sample [`Segment`] value onto a [`Component`].
pub(crate) fn sample_component_keyframes<Source, Target>(
    field: Field<Source, Target>,
) -> ScheduleConfigs<ScheduleSystem>
where
    Source: Component<Mutability = Mutable>,
    Target: Interpolation + Clone + ThreadSafe,
{
    let field_hash = field.to_hash();

    let system =
        move |mut sampler: SegmentSampler<Source, Target>,
              mut q_comps: Query<&mut Source>|
              -> Result {
            sampler.sample_keyframes(
                field_hash,
                |target, action_target, accessor| {
                    let mut comp =
                        q_comps.get_mut(action_target.entity())?;

                    *accessor.get_mut(&mut comp) = target;
                    Ok(())
                },
            )?;

            Ok(())
        };

    system.into_configs()
}

/// Sample [`Segment`] value onto an [`Asset`].
pub(crate) fn sample_asset_keyframes<Source, Target>(
    field: Field<Source::Asset, Target>,
) -> ScheduleConfigs<ScheduleSystem>
where
    Source: AsAssetId,
    Target: Interpolation + Clone + ThreadSafe,
{
    let field_hash = field.to_hash();

    let system =
        move |mut sampler: SegmentSampler<Source::Asset, Target>,
              q_comps: Query<&Source>,
              mut assets: ResMut<Assets<Source::Asset>>|
              -> Result {
            sampler.sample_keyframes(
                field_hash,
                |target, action_target, accessor| {
                    let comp = q_comps.get(action_target.entity())?;
                    let asset = assets
                        .get_mut(comp.as_asset_id())
                        .ok_or(format!(
                        "Can't get asset for {field_hash:?}, id: {}",
                        comp.as_asset_id()
                    ))?;

                    *accessor.get_mut(asset) = target;
                    Ok(())
                },
            )?;

            Ok(())
        };

    system.into_configs()
}

type SegmentSamplerQuery<'w, 's, Target> = Query<
    'w,
    's,
    (
        &'static Segment<Target>,
        Option<&'static Interp<Target>>,
        Option<&'static Ease>,
        &'static ActionTarget,
        &'static SampleType,
        &'static FieldHash,
        Entity,
    ),
>;

#[derive(SystemParam)]
pub(crate) struct SegmentSampler<'w, 's, Source, Target>
where
    Target: ThreadSafe,
    Source: ThreadSafe,
{
    commands: Commands<'w, 's>,
    q_segments: SegmentSamplerQuery<'w, 's, Target>,
    q_accessors:
        Query<'w, 's, &'static FieldAccessor<Source, Target>>,
    field_map: Res<'w, FieldMap>,
}

impl<Source, Target> SegmentSampler<'_, '_, Source, Target>
where
    Source: ThreadSafe,
    Target: Interpolation + Clone + ThreadSafe,
{
    /// Sample [`Segment`]s with the [`SampleType`] component.
    pub(crate) fn sample_keyframes(
        &mut self,
        target_field_hash: FieldHash,
        mut apply_sample: impl FnMut(
            Target,
            &ActionTarget,
            &FieldAccessor<Source, Target>,
        ) -> Result,
    ) -> Result {
        for (
            segment,
            interp,
            ease,
            action_target,
            sample_type,
            field_hash,
            entity,
        ) in self.q_segments.iter()
        {
            // Check for field hash eligibility.
            if field_hash != &target_field_hash {
                continue;
            }

            // Remove marker component so that sampling will not happen
            // in the next frame if it's not needed.
            self.commands.entity(entity).remove::<SampleType>();

            let accessor = self.q_accessors.get(
                *self.field_map.get(&target_field_hash).ok_or(
                    format!(
                        "No FieldAccessor for {target_field_hash:?}"
                    ),
                )?,
            )?;

            let target = match sample_type {
                SampleType::Start => segment.start.clone(),
                SampleType::End => segment.end.clone(),
                SampleType::Interp(mut percent) => {
                    if let Some(ease) = ease {
                        percent = ease(percent);
                    }

                    if let Some(interp) = interp {
                        interp(&segment.start, &segment.end, percent)
                    } else {
                        Target::interp(
                            &segment.start,
                            &segment.end,
                            percent,
                        )
                    }
                }
            };

            apply_sample(target, action_target, accessor)?;
        }

        Ok(())
    }
}

/// Bake [`Action`]s into [`Segment`]s using the `Source` component
/// as the starting point.
pub(crate) fn bake_component_actions<Source, Target>(
    field: Field<Source, Target>,
) -> impl ObserverSystem<OnInsert, Tracks>
where
    Source: Component,
    Target: ThreadSafe + Clone,
{
    let field_hash = field.to_hash();

    let system = move |trigger: Trigger<OnInsert, Tracks>,
                       mut baker: ActionBaker<Source, Target>,
                       q_comps: Query<&Source>|
          -> Result {
        let sequence_id = trigger.target();

        baker.bake_actions(
            sequence_id,
            field_hash,
            |action_target| {
                let comp = q_comps.get(action_target)?;
                Ok(comp)
            },
        )?;

        Ok(())
    };

    IntoObserverSystem::into_system(system)
}

/// Bake [`Action`]s into [`Segment`]s using the `Source::Asset` asset
/// as the starting point.
pub(crate) fn bake_asset_actions<Source, Target>(
    field: Field<Source::Asset, Target>,
) -> impl ObserverSystem<OnInsert, Tracks>
where
    Source: AsAssetId,
    Target: ThreadSafe + Clone,
{
    let field_hash = field.to_hash();

    let system =
        move |trigger: Trigger<OnInsert, Tracks>,
              mut baker: ActionBaker<Source::Asset, Target>,
              q_comps: Query<&Source>,
              assets: Res<Assets<Source::Asset>>|
              -> Result {
            let sequence_id = trigger.target();

            baker.bake_actions(
                sequence_id,
                field_hash,
                |action_target| {
                    let comp = q_comps.get(action_target)?;

                    let asset = assets
                        .get(comp.as_asset_id())
                        .ok_or(format!(
                        "Can't get asset for {field_hash:?}, id: {}",
                        comp.as_asset_id()
                    ))?;

                    Ok(asset)
                },
            )?;

            Ok(())
        };

    IntoObserverSystem::into_system(system)
}

/// System parameters needed to bake [`Action`]s into [`Segment`]s.
#[derive(SystemParam)]
pub(crate) struct ActionBaker<'w, 's, Source, Target>
where
    Source: 'static,
    Target: 'static,
{
    commands: Commands<'w, 's>,
    q_sequences: Query<'w, 's, (&'static Sequence, &'static Tracks)>,
    q_accessors:
        Query<'w, 's, &'static FieldAccessor<Source, Target>>,
    q_actions: Query<'w, 's, &'static Action<Target>>,
    field_map: Res<'w, FieldMap>,
}

impl<Source, Target> ActionBaker<'_, '_, Source, Target>
where
    Source: 'static,
    Target: Clone + ThreadSafe,
{
    /// Bake [`Action`]s into [`Segment`]s if the `field_hash` matches.
    pub(crate) fn bake_actions<'a>(
        &mut self,
        sequence_id: Entity,
        field_hash: FieldHash,
        source_ref: impl Fn(Entity) -> Result<&'a Source>,
    ) -> Result {
        let (sequence, tracks) = self.q_sequences.get(sequence_id)?;

        for (track_key, track) in tracks.iter() {
            // Make sure that the field hash is the same.
            if track_key.field_hash() != &field_hash {
                // Safely skip if it's not the same.
                continue;
            }

            let accessor = self.q_accessors.get(
                *self.field_map.get(&field_hash).ok_or(format!(
                    "No FieldRef for {field_hash:?}"
                ))?,
            )?;

            let mut value = accessor
                .get_ref(source_ref(track_key.action_target())?)
                .clone();

            for span in
                track.span_ids().iter().map(|i| &sequence.spans[*i])
            {
                let action_id = span.action_id();
                let action = self.q_actions.get(action_id)?;

                // Update field to the next value using action.
                let end_value = action(&value);

                self.commands
                    .entity(action_id)
                    .insert(Segment::new(value, end_value.clone()));

                value = end_value;
            }
        }

        Ok(())
    }
}

/// Determines how a [`Segment`] should be sampled.
#[derive(Component, Debug, Clone, Copy)]
#[component(storage = "SparseSet", immutable)]
pub enum SampleType {
    Start,
    End,
    Interp(f32),
}

#[derive(Component)]
pub struct Segment<T> {
    /// The starting value in the segment.
    start: T,
    /// The ending value in the segment.
    end: T,
}

impl<T> Segment<T> {
    pub fn new(start: T, end: T) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> &T {
        &self.start
    }

    pub fn end(&self) -> &T {
        &self.end
    }
}

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub struct Range {
    begin: f32,
    end: f32,
}

impl Range {
    /// Calculate if 2 [`Range`]s overlap.
    pub fn overlap(&self, other: &Self) -> bool {
        self.begin <= other.end && other.begin <= self.end
    }
}
