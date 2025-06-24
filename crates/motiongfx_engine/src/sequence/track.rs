use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use nonempty::NonEmpty;

use crate::action::ActionTarget;
use crate::field::FieldHash;

// For docs.
#[allow(unused_imports)]
use crate::action::ActionSpan;

use super::keyframe::BakeKeyframe;
use super::Sequence;

pub(super) struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(generate_tracks);
    }
}

fn generate_tracks(
    trigger: Trigger<OnInsert, Sequence>,
    mut commands: Commands,
    q_sequences: Query<&Sequence>,
    q_actions: Query<(&FieldHash, &ActionTarget)>,
) -> Result {
    let sequence_id = trigger.target();
    let sequence = q_sequences.get(sequence_id)?;

    let mut tracks = Tracks::default();
    let mut track_ids = Vec::new();

    for (i, span) in sequence.spans.iter().enumerate() {
        let action_id = span.action_id();
        let (&field_hash, &action_target) =
            q_actions.get(action_id)?;

        let track_key = TrackKey {
            action_target,
            field_hash,
        };

        match tracks.get_mut(&track_key) {
            Some(track) => {
                track.span_ids.push(i);
            }
            None => {
                let track_id = commands
                    .spawn((SequenceTarget(sequence_id), track_key))
                    .id();

                tracks.insert(
                    track_key,
                    Track {
                        span_ids: NonEmpty::new(i),
                        track_id,
                    },
                );

                track_ids.push(track_id);
            }
        }
    }

    commands.entity(sequence_id).insert(tracks);

    // Bake keyframes only after the `Tracks` component is inserted.
    for track_id in track_ids {
        commands.entity(track_id).trigger(BakeKeyframe);
    }

    Ok(())
}

/// Stores all uniquely identified tracks in the [`Sequence`],
/// mapped by a unique [`TrackKey`].
#[derive(Component, Deref, DerefMut, Default, Debug, Clone)]
#[component(immutable)]
pub struct Tracks(HashMap<TrackKey, Track>);

// /// Maps the track entity to their respective [`TrackKey`].
// #[derive(Component, Deref, DerefMut, Default, Debug, Clone)]
// #[component(immutable)]
// pub struct TrackKeyKap(EntityHashMap<TrackKey>);

/// Stores the keys required to uniquely identify a track.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[component(immutable)]
pub struct TrackKey {
    /// The target entity that will be animated.
    action_target: ActionTarget,
    /// The target field of the entity that will be animated.
    field_hash: FieldHash,
}

impl TrackKey {
    /// Get the target entity that will be animated.
    pub fn action_target(&self) -> Entity {
        self.action_target.entity()
    }

    /// Get the target field of the entity that will be animated.
    pub fn field_hash(&self) -> &FieldHash {
        &self.field_hash
    }
}

#[derive(Debug, Clone)]
pub struct Track {
    /// The [`ActionSpan`] indices in the [`Sequence`].
    /// Indices should be in ascending order.
    span_ids: NonEmpty<usize>,
    /// The target entity that stores the [`Keyframe`]s of the track.
    track_id: Entity,
}

impl Track {
    /// Get the [`ActionSpan`] indices in the [`Sequence`].
    /// Indices should be in ascending order.
    pub fn span_ids(&self) -> &NonEmpty<usize> {
        &self.span_ids
    }

    /// Get the target entity that stores the [`Keyframe`]s of the track.
    pub fn track_id(&self) -> Entity {
        self.track_id
    }
}

/// The [`Track`] entities that belongs to the
/// [`Sequence`] that is attached to this entity.
#[derive(Component, Reflect, Deref, Clone)]
#[reflect(Component)]
#[relationship_target(relationship = SequenceTarget, linked_spawn)]
pub struct SequenceTracks(Vec<Entity>);

/// The [`Sequence`] entity that the [`Track`] in this entity belongs to.
#[derive(Component, Reflect, Deref, Clone)]
#[reflect(Component)]
#[relationship(relationship_target = SequenceTracks)]
pub struct SequenceTarget(Entity);
