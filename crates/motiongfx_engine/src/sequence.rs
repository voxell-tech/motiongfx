use bevy::asset::AsAssetId;
use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use segment::{
    bake_asset_keyframes, bake_component_keyframes,
    sample_asset_keyframes, sample_component_keyframes,
};
use smallvec::SmallVec;

use crate::action::ActionSpan;
use crate::field::{FieldBundle, RegisterFieldAppExt};
use crate::prelude::Interpolation;
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
            (
                update_target_time.in_set(MotionGfxSet::TargetTime),
                update_curr_time.in_set(MotionGfxSet::CurrentTime),
            ),
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
        .add_observer(bake_component_keyframes(field_bundle.field))
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
        .add_observer(bake_asset_keyframes::<Source, _>(
            field_bundle.field,
        ))
        .register_field(field_bundle)
    }
}

/// Update [`SequenceController::target_time`] based on [`SequencePlayer::time_scale`].
fn update_target_time(
    mut q_sequences: Query<(
        &Sequence,
        &mut SequenceController,
        &SequencePlayer,
    )>,
    time: Res<Time>,
) {
    for (sequence, mut sequence_controller, sequence_player) in
        q_sequences.iter_mut()
    {
        sequence_controller.target_time = f32::clamp(
            sequence_controller.target_time
                + time.delta_secs() * sequence_player.time_scale,
            0.0,
            sequence.duration(),
        );
    }
}

/// Safely update [`SequenceController::curr_time`] after performing
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

// TODO: Remove this as a component?
// Add type wrappers to them. e.g. Slide, Timeline..

/// A group of actions in chronological order.
#[derive(Component, Default, Clone)]
#[require(SequenceController)]
#[component(immutable)]
pub struct Sequence {
    duration: f32,
    /// Stores the [`ActionSpan`]s that makes up the sequence.
    pub(crate) spans: SmallVec<[ActionSpan; 1]>,
}

impl Sequence {
    pub fn single(span: ActionSpan) -> Self {
        let duration = span.duration();
        let mut spans = SmallVec::new();
        spans.push(span);

        Self { spans, duration }
    }

    pub fn empty(duration: f32) -> Self {
        Self {
            duration,
            ..default()
        }
    }

    pub fn with_slide_index(mut self, index: u32) -> Self {
        for span in self.spans.iter_mut() {
            span.set_slide_index(index);
        }

        self
    }

    pub fn set_slide_index(&mut self, index: u32) {
        for span in self.spans.iter_mut() {
            span.set_slide_index(index);
        }
    }

    #[inline]
    pub fn duration(&self) -> f32 {
        self.duration
    }
}

/// Plays the [`Sequence`] component attached to this entity
/// through `target_time` manipulation.
#[derive(Component, Default)]
pub struct SequenceController {
    /// The current time.
    curr_time: f32,
    /// Target time to reach (and not exceed).
    pub target_time: f32,
    /// Target slide index to reach (and not exceed).
    pub target_slide_index: usize,
}

impl SequenceController {
    /// Get the current time.
    pub fn curr_time(&self) -> f32 {
        self.curr_time
    }
}

/// Manipulates the `target_time` variable of the [`SequenceController`]
/// component attached to this entity with a `time_scale`.
#[derive(Component, Default)]
pub struct SequencePlayer {
    pub is_playing: bool,
    pub time_scale: f32,
}

impl SequencePlayer {
    pub fn play(&mut self) {
        self.is_playing = true;
    }

    pub fn pause(&mut self) {
        self.is_playing = false;
    }
}

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

impl MultiSeqOrd for &[Sequence] {
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
pub fn chain(sequences: &[Sequence]) -> Sequence {
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
pub fn all(sequences: &[Sequence]) -> Sequence {
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
pub fn any(sequences: &[Sequence]) -> Sequence {
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
pub fn flow(t: f32, sequences: &[Sequence]) -> Sequence {
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
