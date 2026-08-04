use bevy::prelude::*;

/// The scrubbable timeline track: a plain node sized to the track's
/// duration (`PIXELS_PER_SECOND` per second), so a clip at time `t`
/// sits at `t * PIXELS_PER_SECOND` from its left edge.
///
/// Scrubbing is driven by pointer observers on this node (see
/// the editor's track pointer observers) rather than a
/// headless `Slider`: a scrub can only *begin* from a press that
/// actually lands inside the track, so it can't be started from
/// elsewhere in the window.
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
