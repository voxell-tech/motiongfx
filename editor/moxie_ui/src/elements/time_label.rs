use crate::reactive::BevyHost;
use bevy::prelude::*;
use bevy_fynix::EntityExt as _;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::{Build, Patch};

use super::Label;

/// A time reading on the time axis, placing above the mark it reads
/// for.
#[derive(Element)]
pub struct TimeLabel {
    #[elem(child)]
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
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((self.node(), Pickable::IGNORE));
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: TimeLabelField,
    ) {
        match field {
            TimeLabelField::X => {
                patch.insert(self.node());
            }
        }
    }
}
