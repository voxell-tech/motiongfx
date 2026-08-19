use crate::reactive::{BevyHost, BevyUi};
use bevy::prelude::*;
use fynix_mock::element::{Element, ElementVisual};

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
    fn build_fields(&self, ui: &mut BevyUi<'_>) {
        let node = ui.parent();
        let world = &mut *ui.world;

        world.entity_mut(node).insert(self.node());
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: TimelineTrackField,
    ) {
        match field {
            TimelineTrackField::Width => {
                world.entity_mut(node).insert(self.node());
            }
        }
    }
}
