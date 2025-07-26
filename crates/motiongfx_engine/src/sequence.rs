use bevy::ecs::component::{ComponentHooks, Immutable, StorageType};
use bevy::prelude::*;
use smallvec::SmallVec;

use crate::action::ActionSpan;
use crate::MotionGfxSet;

// For docs.
#[allow(unused_imports)]
use super::action::Action;

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

/// Safely update [`SequenceController::curr_time`] after sampling
/// all the necessary actions.
fn update_curr_time(
    mut q_sequences: Query<(&Sequence, &mut SequenceController)>,
) {
    for (sequence, mut controller) in q_sequences.iter_mut() {
        let controller = controller.bypass_change_detection();

        controller.target_time =
            controller.target_time.clamp(0.0, sequence.duration());

        controller.curr_time = controller.target_time;
    }
}

/// A group of actions in chronological order.
///
/// A [`SequenceController`] will also be automatically inserted
/// through the `on_insert` hook.
#[derive(Default, Debug, Clone)]
pub struct Sequence {
    /// Stores the [`ActionSpan`]s that makes up the sequence.
    pub(crate) spans: SmallVec<[ActionSpan; 1]>,
    /// The duration of the entire sequence, accumulated from `spans`.
    duration: f32,
}

impl Sequence {
    #[must_use]
    pub fn single(span: ActionSpan) -> Self {
        Self {
            duration: span.duration(),
            spans: SmallVec::from_buf([span]),
        }
    }

    #[must_use]
    pub fn empty(duration: f32) -> Self {
        Self {
            duration,
            ..default()
        }
    }

    #[inline(always)]
    #[must_use]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn chain(&mut self, sequence: Self) -> &mut Self {
        for span in sequence.spans {
            self.spans.push(
                span.with_start_time(
                    span.start_time() + self.duration,
                ),
            );
        }

        self.duration += sequence.duration;

        self
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

impl IntoIterator for Sequence {
    type Item = Self;

    type IntoIter = core::iter::Once<Self>;

    fn into_iter(self) -> Self::IntoIter {
        core::iter::once(self)
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
