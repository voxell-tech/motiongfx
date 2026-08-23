use bevy::prelude::*;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

/// One mark on the time axis.
#[derive(Element, OverrideDefault, Lenz)]
pub struct TimeTick {
    /// Pixels from the time axis's left edge.
    pub x: f32,
    /// Grown upward from the time axis's bottom edge, so marks of
    /// different lengths share a baseline.
    #[default(4.0)]
    pub height: f32,
    #[default(Color::srgba(1.0, 1.0, 1.0, 0.25))]
    pub color: Color,
}

impl TimeTick {
    fn node(&self) -> Node {
        Node {
            position_type: PositionType::Absolute,
            left: px(self.x),
            bottom: px(0),
            width: px(1),
            height: px(self.height),
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for TimeTick {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert((
            self.node(),
            BackgroundColor(self.color),
            Pickable::IGNORE,
        ));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: TimeTickField,
    ) {
        match field {
            TimeTickField::Color => {
                world
                    .entity_mut(node)
                    .insert(BackgroundColor(self.color));
            }
            TimeTickField::X | TimeTickField::Height => {
                world.entity_mut(node).insert(self.node());
            }
        }
    }
}
