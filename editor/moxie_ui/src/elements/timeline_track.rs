use crate::reactive::FynixBuild;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::patch;

/// The scrubbable timeline track: a plain node sized to the track's
/// duration. The consuming app resolves its own pixels per second
/// scale and passes the result as `width`, so a clip at time `t` sits
/// at `t * pixels_per_second` from the track's left edge.
#[element(build = Self::build)]
pub struct TimelineTrack {
    #[elem(patch = patch::track_width)]
    pub width: Val,
}

impl TimelineTrack {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert(Node {
            position_type: PositionType::Relative,
            height: percent(100),
            ..default()
        });
    }
}
