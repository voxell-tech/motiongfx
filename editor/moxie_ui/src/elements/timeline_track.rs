use crate::reactive::BevyHost;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::{Build, Patch};

/// The scrubbable timeline track: a plain node sized to the track's
/// duration. The consuming app resolves its own pixels per second
/// scale and passes the result as `width`, so a clip at time `t` sits
/// at `t * pixels_per_second` from the track's left edge.
#[derive(Element)]
pub struct TimelineTrack {
    pub width: f32,
}

impl TimelineTrack {
    fn node(&self) -> Node {
        Node {
            position_type: PositionType::Relative,
            width: px(self.width),
            min_width: px(self.width),
            height: percent(100),
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for TimelineTrack {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert(self.node());
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: TimelineTrackField,
    ) {
        match field {
            TimelineTrackField::Width => {
                patch.insert(self.node());
            }
        }
    }
}
