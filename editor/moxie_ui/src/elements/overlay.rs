use crate::reactive::BevyHost;
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy_fynix::EntityExt as _;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::{Build, Patch};

/// A node the size of the window, for something that positions itself
/// against the window rather than against a parent.
#[derive(Element)]
pub struct Overlay {
    /// Whether the pointer sees it. Off unless it is there to catch
    /// something: seen, it is the target of every press, and a press
    /// on it takes focus from whatever is underneath.
    pub catches: bool,
    pub z: i32,
}

impl Overlay {
    /// It never blocks. What it catches, it catches by being seen.
    fn pickable(&self) -> Pickable {
        Pickable {
            should_block_lower: false,
            is_hoverable: self.catches,
        }
    }
}

impl ElementVisual<BevyHost> for Overlay {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
            self.pickable(),
            GlobalZIndex(self.z),
        ));
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: OverlayField,
    ) {
        match field {
            OverlayField::Catches => {
                patch.insert(self.pickable());
            }
            OverlayField::Z => {
                patch.insert(GlobalZIndex(self.z));
            }
        }
    }
}
