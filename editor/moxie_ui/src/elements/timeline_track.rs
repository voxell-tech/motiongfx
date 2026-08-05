use bevy::prelude::*;

/// The scrubbable timeline track: a plain node sized to the track's
/// duration. The consuming app resolves its own pixels-per-second
/// scale and passes the result as `width`, so a clip at time `t` sits
/// at `t * pixels_per_second` from the track's left edge.
#[derive(SceneComponent, Default, Clone)]
#[scene(TimelineTrackProps)]
pub struct TimelineTrack;

#[derive(Default)]
pub struct TimelineTrackProps {
    pub width: f32,
}

impl TimelineTrack {
    fn scene(
        TimelineTrackProps { width }: TimelineTrackProps,
    ) -> impl Scene {
        bsn! {
            TimelineTrack
            Node {
                position_type: PositionType::Relative,
                width: Val::Px(width),
                min_width: Val::Px(width),
                height: Val::Percent(100.0),
            }
        }
    }
}
