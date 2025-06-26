use bevy::asset::AsAssetId;
use bevy::ecs::component::{
    ComponentHooks, Immutable, Mutable, StorageType,
};
use bevy::prelude::*;
use segment::{
    bake_asset_actions, bake_component_actions,
    sample_asset_keyframes, sample_component_keyframes,
};
use smallvec::SmallVec;

use crate::action::ActionSpan;
use crate::field::{FieldBundle, RegisterFieldAppExt};
use crate::interpolation::Interpolation;
use crate::{MotionGfxSet, ThreadSafe};

pub mod segment;
pub mod track;

pub(super) struct SequencePlugin;

impl Plugin for SequencePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            track::TrackPlugin,
            segment::KeyframePlugin,
        ));

        app.add_systems(
            PostUpdate,
            update_curr_time.in_set(MotionGfxSet::CurrentTime),
        );
    }
}

pub trait AnimateAppExt {
    fn animate_component<Source, Target>(
        &mut self,
        field_bundle: FieldBundle<Source, Target>,
    ) -> &mut Self
    where
        Source: Component<Mutability = Mutable>,
        Target: Interpolation + Clone + ThreadSafe;

    fn animate_asset<Source, Target>(
        &mut self,
        field_bundle: FieldBundle<Source::Asset, Target>,
    ) -> &mut Self
    where
        Source: AsAssetId,
        Target: Interpolation + Clone + ThreadSafe;
}

impl AnimateAppExt for App {
    fn animate_component<Source, Target>(
        &mut self,
        field_bundle: FieldBundle<Source, Target>,
    ) -> &mut Self
    where
        Source: Component<Mutability = Mutable>,
        Target: Interpolation + Clone + ThreadSafe,
    {
        self.add_systems(
            PostUpdate,
            sample_component_keyframes(field_bundle.field)
                .in_set(MotionGfxSet::Sample),
        )
        .add_observer(bake_component_actions(field_bundle.field))
        .register_field(field_bundle)
    }

    fn animate_asset<Source, Target>(
        &mut self,
        field_bundle: FieldBundle<Source::Asset, Target>,
    ) -> &mut Self
    where
        Source: AsAssetId,
        Target: Interpolation + Clone + ThreadSafe,
    {
        self.add_systems(
            PostUpdate,
            sample_asset_keyframes::<Source, _>(field_bundle.field)
                .in_set(MotionGfxSet::Sample),
        )
        .add_observer(bake_asset_actions::<Source, _>(
            field_bundle.field,
        ))
        .register_field(field_bundle)
    }
}

/// Safely update [`SequenceController::curr_time`] after sampling
/// all the necessary actions.
fn update_curr_time(
    mut q_sequences: Query<(&Sequence, &mut SequenceController)>,
) {
    for (sequence, mut controller) in q_sequences.iter_mut() {
        let controller = controller.bypass_change_detection();

        controller.target_time = f32::clamp(
            controller.target_time,
            0.0,
            sequence.duration(),
        );

        controller.curr_time = controller.target_time;
    }
}

/// A group of actions in chronological order.
///
/// A [`SequenceController`] will also be automatically inserted
/// through the `on_insert` hook.
#[derive(Default, Clone)]
pub struct Sequence {
    /// Stores the [`ActionSpan`]s that makes up the sequence.
    pub(crate) spans: SmallVec<[ActionSpan; 1]>,
    /// The duration of the entire sequence, accumulated from `spans`.
    duration: f32,
}

impl Sequence {
    pub fn single(span: ActionSpan) -> Self {
        Self {
            duration: span.duration(),
            spans: [span].into(),
        }
    }

    pub fn empty(duration: f32) -> Self {
        Self {
            duration,
            ..default()
        }
    }

    #[inline(always)]
    pub fn duration(&self) -> f32 {
        self.duration
    }
}

impl Component for Sequence {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    type Mutability = Immutable;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_insert(|mut world, context| {
            let action_ids = world
                .get::<Self>(context.entity)
                // SAFETY: Hook should only trigger after the component is inserted.
                .unwrap()
                .spans
                .iter()
                .map(|span| span.action_id())
                .collect::<Vec<_>>();

            for action_id in action_ids {
                world
                    .commands()
                    .entity(action_id)
                    .insert(TargetSequence(context.entity));
            }

            // Re-inserts a new SequenceController.
            world
                .commands()
                .entity(context.entity)
                .insert(SequenceController::default());
        });
    }
}

/// Plays the [`Sequence`] component attached to this entity
/// through `target_time` manipulation.
#[derive(Component, Default)]
pub struct SequenceController {
    /// The current time.
    curr_time: f32,
    /// The target time to reach (and not exceed).
    pub target_time: f32,
}

impl SequenceController {
    /// Get the current time.
    pub fn curr_time(&self) -> f32 {
        self.curr_time
    }
}

/// [`Action`]s that are related to this [`Sequence`].
#[derive(Component, Reflect, Deref, Clone)]
#[reflect(Component)]
#[relationship_target(relationship = TargetSequence, linked_spawn)]
pub struct SequenceActions(Vec<Entity>);

/// The target [`Sequence`] that this [`Action`] belongs to.
#[derive(
    Component, Reflect, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
#[reflect(Component)]
#[relationship(relationship_target = SequenceActions)]
pub struct TargetSequence(Entity);

// SEQUENCE ORDERING FUNCTIONS

pub trait MultiSeqOrd {
    /// Run one [`Sequence`] after another.
    fn chain(self) -> Sequence;
    /// Run all [`Sequence`]s concurrently and wait for all of them to finish.
    fn all(self) -> Sequence;
    /// Run all [`Sequence`]s concurrently and wait for any of them to finish.
    fn any(self) -> Sequence;
    /// Run one [`Sequence`] after another with a fixed delay time.
    fn flow(self, delay: f32) -> Sequence;
}

impl<T: IntoIterator<Item = Sequence>> MultiSeqOrd for T {
    fn chain(self) -> Sequence {
        chain(self)
    }

    fn all(self) -> Sequence {
        all(self)
    }

    fn any(self) -> Sequence {
        any(self)
    }

    fn flow(self, t: f32) -> Sequence {
        flow(t, self)
    }
}

pub trait SingleSeqOrd {
    /// Run a [`Sequence`] after a fixed delay time.
    fn delay(self, t: f32) -> Sequence;
}

impl SingleSeqOrd for Sequence {
    fn delay(self, t: f32) -> Sequence {
        delay(t, self)
    }
}

/// Run one [`Sequence`] after another.
pub fn chain(
    sequences: impl IntoIterator<Item = Sequence>,
) -> Sequence {
    let mut final_sequence = Sequence::default();
    let mut chain_duration = 0.0;

    for sequence in sequences {
        for span in &sequence.spans {
            final_sequence.spans.push(
                span.with_start_time(
                    span.start_time() + chain_duration,
                ),
            );
        }

        chain_duration += sequence.duration;
    }

    final_sequence.duration = chain_duration;
    final_sequence
}

/// Run all [`Sequence`]s concurrently and wait for all of them to finish.
pub fn all(
    sequences: impl IntoIterator<Item = Sequence>,
) -> Sequence {
    let mut final_sequence = Sequence::default();
    let mut max_duration = 0.0;

    for sequence in sequences {
        for span in &sequence.spans {
            final_sequence.spans.push(*span);
        }

        max_duration = f32::max(max_duration, sequence.duration);
    }

    final_sequence.duration = max_duration;
    final_sequence
}

/// Run all [`Sequence`]s concurrently and wait for any of them to finish.
pub fn any(
    sequences: impl IntoIterator<Item = Sequence>,
) -> Sequence {
    let mut final_sequence = Sequence::default();
    let mut min_duration = 0.0;

    for action_grp in sequences {
        for span in &action_grp.spans {
            final_sequence.spans.push(*span);
        }

        min_duration = f32::min(min_duration, action_grp.duration);
    }

    final_sequence.duration = min_duration;
    final_sequence
}

/// Run one [`Sequence`] after another with a fixed delay time.
pub fn flow(
    t: f32,
    sequences: impl IntoIterator<Item = Sequence>,
) -> Sequence {
    let mut final_sequence = Sequence::default();
    let mut flow_duration = 0.0;
    let mut final_duration = 0.0;

    for sequence in sequences {
        for span in &sequence.spans {
            final_sequence.spans.push(
                span.with_start_time(
                    span.start_time() + flow_duration,
                ),
            );
        }

        flow_duration += t;
        final_duration = f32::max(
            final_duration,
            flow_duration + sequence.duration,
        );
    }

    final_sequence.duration = final_duration;
    final_sequence
}

/// Run a [`Sequence`] after a fixed delay time.
pub fn delay(t: f32, sequence: Sequence) -> Sequence {
    let mut final_sequence = Sequence::default();

    for span in &sequence.spans {
        final_sequence
            .spans
            .push(span.with_start_time(span.start_time() + t));
    }

    final_sequence.duration = sequence.duration + t;
    final_sequence
}
