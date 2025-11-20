use bevy::prelude::*;
use bevy_motiongfx::prelude::*;

pub fn timeline_movement(
    mut motiongfx: ResMut<MotionGfxWorld>,
    mut q_timelines: Query<(&TimelineId, &mut RealtimePlayer)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) -> Result {
    for (id, mut player) in q_timelines.iter_mut() {
        if let Some(timeline) = motiongfx.get_timeline_mut(id) {
            if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight])
            {
                player.set_playing(false);
                let target_time =
                    timeline.target_time() + time.delta_secs();
                timeline.set_target_time(target_time);
            }

            if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
                player.set_playing(false);
                let target_time =
                    timeline.target_time() - time.delta_secs();
                timeline.set_target_time(target_time);
            }

            if keys.just_pressed(KeyCode::Space) {
                if keys.pressed(KeyCode::ShiftLeft) {
                    player.set_time_scale(-1.0);
                } else {
                    player.set_time_scale(1.0);
                }
                player.set_playing(true);
            }

            if keys.just_pressed(KeyCode::Escape) {
                player.set_playing(false);
            }
        }
    }

    Ok(())
}
