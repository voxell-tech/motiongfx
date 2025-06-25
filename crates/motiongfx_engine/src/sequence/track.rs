use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use nonempty::NonEmpty;

use crate::action::ActionTarget;
use crate::field::FieldHash;

// For docs.
#[allow(unused_imports)]
use super::segment::Segment;
#[allow(unused_imports)]
use crate::action::ActionSpan;

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
                track.push_span(i, span);
            }
            None => {
                tracks.insert(track_key, Track::new(i, span));
            }
        }
    }

    commands.entity(sequence_id).insert(tracks);

    Ok(())
}

/// Stores all uniquely identified tracks in the [`Sequence`],
/// mapped by a unique [`TrackKey`].
#[derive(Component, Deref, DerefMut, Default, Debug, Clone)]
#[component(immutable)]
pub struct Tracks(HashMap<TrackKey, Track>);

/// Stores the keys required to uniquely identify a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    start_time: f32,
    end_time: f32,
}

impl Track {
    fn new(span_id: usize, span: &ActionSpan) -> Self {
        Self {
            span_ids: NonEmpty::new(span_id),
            start_time: span.start_time(),
            end_time: span.end_time(),
        }
    }

    fn push_span(&mut self, span_id: usize, span: &ActionSpan) {
        self.span_ids.push(span_id);
        // Push the end time further down.
        self.end_time = span.end_time();
    }

    /// Get the [`ActionSpan`] indices in the [`Sequence`].
    /// Indices should always be in ascending order.
    #[inline(always)]
    pub fn span_ids(&self) -> &NonEmpty<usize> {
        &self.span_ids
    }

    #[inline(always)]
    /// Get the start time of the track.
    pub fn start_time(&self) -> f32 {
        self.start_time
    }

    #[inline(always)]
    /// Get the end time of the track.
    pub fn end_time(&self) -> f32 {
        self.end_time
    }
}
