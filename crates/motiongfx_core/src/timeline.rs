use crate::sequence::Sequence;
use bevy_ecs::prelude::*;
use bevy_time::Time;

#[derive(Resource)]
pub struct Timeline {
    pub is_playing: bool,
    pub time_scale: f32,
    pub curr_time: f32,
    pub target_time: f32,
}

impl Timeline {
    pub fn new() -> Self {
        Self {
            is_playing: false,
            time_scale: 1.0,
            curr_time: 0.0,
            target_time: 0.0,
        }
    }
}

/// Safely update the timings in the `Timeline` after performing all the necessary actions.
pub(crate) fn timeline_update_system(
    mut timeline: ResMut<Timeline>,
    sequence: Res<Sequence>,
    time: Res<Time>,
) {
    timeline.curr_time = timeline.target_time;

    if timeline.is_playing {
        timeline.target_time += time.delta_seconds() * timeline.time_scale;
    }

    timeline.target_time = f32::clamp(timeline.target_time, 0.0, sequence.duration());
}
