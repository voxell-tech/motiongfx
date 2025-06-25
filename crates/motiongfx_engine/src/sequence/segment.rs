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

use super::track::{SequenceTarget, TrackKey, Tracks};
use super::{Sequence, SequenceController};

pub(super) struct KeyframePlugin;

impl Plugin for KeyframePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            mark_tracks_for_sampling.in_set(MotionGfxSet::MarkTrack),
        );
    }
}

/// Mark tracks that overlaps with the current and target time
/// from the [`SequenceController`].
fn mark_tracks_for_sampling(
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
                    if index == 0 {
                        let span = &sequence.spans[*span_ids.first()];
                        commands
                            .entity(span.action_id())
                            .insert(SampleType::Start);
                    } else {
                        let span =
                            &sequence.spans[span_ids[index - 1]];
                        commands
                            .entity(span.action_id())
                            .insert(SampleType::End);
                    }
                }
            }
        }
    }
}

/// Sample [`Keyframes`] value onto a [`Component`].
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

/// Sample [`Keyframes`] value onto an [`Asset`].
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

/// Bake [`Action`]s into [`Keyframes`] using the `Source` component
/// as the starting point.
pub(super) fn bake_component_keyframes<Source, Target>(
    field: Field<Source, Target>,
) -> impl ObserverSystem<BakeKeyframe, ()>
where
    Source: Component,
    Target: ThreadSafe + Clone,
{
    let field_hash = field.to_hash();

    let system = move |trigger: Trigger<BakeKeyframe>,
                       mut baker: ActionBaker<Source, Target>,
                       q_comps: Query<&Source>|
          -> Result {
        let track_id = trigger.target();

        baker.bake_actions(
            track_id,
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

/// Bake [`Action`]s into [`Keyframes`] using the `Source::Asset` asset
/// as the starting point.
pub(super) fn bake_asset_keyframes<Source, Target>(
    field: Field<Source::Asset, Target>,
) -> impl ObserverSystem<BakeKeyframe, ()>
where
    Source: AsAssetId,
    Target: ThreadSafe + Clone,
{
    let field_hash = field.to_hash();

    let system =
        move |trigger: Trigger<BakeKeyframe>,
              mut baker: ActionBaker<Source::Asset, Target>,
              q_comps: Query<&Source>,
              assets: Res<Assets<Source::Asset>>|
              -> Result {
            let track_id = trigger.target();
            baker.bake_actions(
                track_id,
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
    q_tracks:
        Query<'w, 's, (&'static TrackKey, &'static SequenceTarget)>,
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
        track_id: Entity,
        field_hash: FieldHash,
        source_ref: impl Fn(Entity) -> Result<&'a Source>,
    ) -> Result {
        let (track_key, sequence_target) =
            self.q_tracks.get(track_id)?;

        // Make sure that the field hash is the same.
        if track_key.field_hash() != &field_hash {
            // Safely skip if it's not the same.
            return Ok(());
        }

        let (sequence, tracks) =
            self.q_sequences.get(sequence_target.entity())?;

        let track = tracks
            .get(track_key)
            .ok_or(format!("No track found for {track_key:?}!"))?;

        let accessor = self.q_accessors.get(
            *self
                .field_map
                .get(&field_hash)
                .ok_or(format!("No FieldRef for {field_hash:?}"))?,
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

        Ok(())
    }
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub(crate) struct SampleKeyframes;

/// Triggers [`bake_component_keyframes()`] and [`bake_asset_keyframes()`].
#[derive(Event)]
pub(crate) struct BakeKeyframe;

// #[derive(Component, Deref, DerefMut, Debug, Clone)]
// #[component(immutable)]
// pub struct Keyframes<T>(NonEmpty<Keyframe<T>>);

// impl<T> Keyframes<T> {
//     pub fn new(first_keyframe: Keyframe<T>) -> Self {
//         Self(NonEmpty::new(first_keyframe))
//     }
// }

// impl<T> Keyframes<T> {
//     pub fn sample(&self, time: f32) -> Sample<'_, T> {
//         let index = self
//             .binary_search_by(|kf| {
//                 if kf.time > time {
//                     Ordering::Greater
//                 } else {
//                     Ordering::Less
//                 }
//             })
//             // SAFETY: Ordering::Equal is never returned.
//             .unwrap_err();

//         if index == 0 {
//             Sample::Single(&self.first().value)
//         } else if index >= self.len() {
//             Sample::Single(&self.last().value)
//         } else {
//             let start = &self[index - 1];
//             let end = &self[index];

//             // An action id is only added at the end keyframe.
//             // See `KeyframeBaker`.
//             match end.action_id {
//                 Some(action_id) => {
//                     let percent =
//                         (time - start.time) / (end.time - start.time);

//                     Sample::Interp {
//                         start: &start.value,
//                         end: &end.value,
//                         action_id,
//                         percent,
//                     }
//                 }
//                 // Interpolation method is unknown without an action id.
//                 //
//                 // This normally happens when there's a time gap
//                 // between Action commands. Which in this case, the start
//                 // and end value should always be the same anyways.
//                 None => Sample::Single(&start.value),
//             }
//         }
//     }
// }

#[derive(Component, Debug, Clone, Copy)]
#[component(storage = "SparseSet", immutable)]
pub enum SampleType {
    Start,
    End,
    Interp(f32),
}

// /// Determines how a value should be sampled.
// ///
// /// Typically used for [`Keyframes::sample()`].
// pub enum Sample<'a, T> {
//     /// A single value that can be sampled directly.
//     Single(&'a T),
//     /// A value pair that needs to be sampled via
//     /// some sort of interpolation.
//     Interp {
//         start: &'a T,
//         end: &'a T,
//         action_id: Entity,
//         percent: f32,
//     },
// }

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

// /// Holds a specific `value` at a given `time`. It might also link
// /// to an action which defines how a [`Sample`] should be interpolated.
// #[derive(Debug, Clone, Copy)]
// pub struct Keyframe<T> {
//     time: f32,
//     value: T,
//     action_id: Option<Entity>,
// }

// impl<T> Keyframe<T> {
//     pub fn new(time: f32, value: T) -> Self {
//         Self {
//             time,
//             value,
//             action_id: None,
//         }
//     }

//     pub fn with_action(mut self, action_id: Entity) -> Self {
//         self.action_id = Some(action_id);
//         self
//     }
// }

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
