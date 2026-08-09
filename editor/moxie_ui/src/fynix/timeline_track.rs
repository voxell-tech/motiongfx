use bevy::prelude::*;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

/// The scrubbable timeline track: a plain node sized to the track's
/// duration. The consuming app resolves its own pixels per second
/// scale and passes the result as `width`, so a clip at time `t` sits
/// at `t * pixels_per_second` from the track's left edge.
#[derive(Element, OverrideDefault, Lenz)]
pub struct TimelineTrack {
    pub width: f32,
}

impl TimelineTrack {
    fn node(&self) -> Node {
        Node {
            position_type: PositionType::Relative,
            width: Val::Px(self.width),
            min_width: Val::Px(self.width),
            height: Val::Percent(100.0),
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for TimelineTrack {
    fn build_fields(&self, world: &mut World, node: Entity) {
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
