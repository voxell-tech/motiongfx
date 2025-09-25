use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_time::prelude::*;
use motiongfx::prelude::*;

use crate::MotionGfxSet;

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            realtime_player_timing.in_set(MotionGfxSet::Controller),
        );
    }
}

fn realtime_player_timing(
    mut q_timelines: Query<(&mut Timeline, &RealtimePlayer)>,
    time: Res<Time>,
) {
    for (mut timeline, player) in q_timelines.iter_mut() {
        if player.is_playing {
            let target_time = timeline.curr_time()
                + player.time_scale * time.delta_secs();

            timeline.set_target_time(target_time);
        }
    }
}

#[derive(Component, Debug)]
pub struct RealtimePlayer {
    /// Determines if the timeline is currently playing.
    pub is_playing: bool,
    /// The time scale of the player. Set this to negative
    /// to play backwards.
    pub time_scale: f32,
}

impl RealtimePlayer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder method for setting [`Timeline::is_playing`].
    pub fn with_playing(mut self, playing: bool) -> Self {
        self.is_playing = playing;
        self
    }

    /// Builder method for setting [`Timeline::time_scale`].
    pub fn with_time_scale(mut self, time_scale: f32) -> Self {
        self.time_scale = time_scale;
        self
    }

    /// Setter method for setting [`Timeline::is_playing`].
    pub fn set_playing(&mut self, playing: bool) -> &mut Self {
        self.is_playing = playing;
        self
    }

    /// Setter method for setting [`Timeline::time_scale`].
    pub fn set_time_scale(&mut self, time_scale: f32) -> &mut Self {
        self.time_scale = time_scale;
        self
    }
}

impl Default for RealtimePlayer {
    fn default() -> Self {
        Self {
            is_playing: false,
            time_scale: 1.0,
        }
    }
}
