use bevy::prelude::*;

use crate::timeline_v2::Timeline;

pub struct CleanupPlugin;

impl Plugin for CleanupPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(cleanup_timeline);
    }
}

// Clean up the actions spawned when creating the [`Timeline`].
fn cleanup_timeline(
    trigger: Trigger<OnReplace, Timeline>,
    mut commands: Commands,
    q_timelines: Query<&Timeline>,
) -> Result {
    let timeline = q_timelines.get(trigger.target())?;

    for track in timeline.tracks() {
        for (_, spans) in track.iter_sequences() {
            for span in spans {
                commands.entity(span.action_id()).despawn();
            }
        }
    }

    Ok(())
}
