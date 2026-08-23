use bevy::prelude::*;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

use super::Label;

/// A time reading on the time axis, placing above the mark it reads
/// for.
#[derive(Element, OverrideDefault, Lenz)]
pub struct TimeLabel {
    #[elem]
    pub label: Label,
    /// Pixels from the time axis's left edge.
    pub x: f32,
}

impl TimeLabel {
    fn node(&self) -> Node {
        Node {
            position_type: PositionType::Absolute,
            left: px(self.x),
            top: px(1),
            width: px(0),
            justify_content: if self.centred() {
                JustifyContent::Center
            } else {
                JustifyContent::FlexStart
            },
            padding: UiRect::left(if self.centred() {
                Val::ZERO
            } else {
                px(3)
            }),
            ..default()
        }
    }

    fn centred(&self) -> bool {
        self.x > 0.0
    }
}

impl ElementVisual<BevyHost> for TimeLabel {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world
            .entity_mut(node)
            .insert((self.node(), Pickable::IGNORE));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: TimeLabelField,
    ) {
        match field {
            TimeLabelField::X => {
                world.entity_mut(node).insert(self.node());
            }
        }
    }
}
