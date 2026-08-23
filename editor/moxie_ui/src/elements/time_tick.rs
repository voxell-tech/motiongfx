use crate::reactive::BevyHost;
use bevy::prelude::*;
use bevy_fynix::EntityExt as _;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::{Build, Patch};

/// One mark on the time axis.
#[derive(Element)]
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
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((
            self.node(),
            BackgroundColor(self.color),
            Pickable::IGNORE,
        ));
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: TimeTickField,
    ) {
        match field {
            TimeTickField::Color => {
                patch.insert(BackgroundColor(self.color));
            }
            TimeTickField::X | TimeTickField::Height => {
                patch.insert(self.node());
            }
        }
    }
}
