use crate::reactive::FynixHost;
use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::ControlOrientation;
use bevy::window::SystemCursorIcon;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;
use fynix::ui::Patch;

use super::patch::{self, node};

const DIVIDER_WIDTH: f32 = 6.0;

/// The draggable line between two panes.
#[element]
pub struct Divider {
    #[elem(patch = thickness)]
    #[default(px(DIVIDER_WIDTH))]
    pub thickness: Val,
    #[elem(patch = orientation)]
    #[default(::Horizontal)]
    pub orientation: ControlOrientation,
    #[elem(patch = patch::background)]
    #[default(Color::srgba(1.0, 1.0, 1.0, 0.08))]
    pub color: Color,
}

/// The axis the line runs along, read back from whichever dimension
/// is the full-length one.
fn horizontal(node: &Node) -> bool {
    node.width == percent(100)
}

fn thickness(patch: &mut Patch<FynixHost>, thickness: &Val) {
    node(patch, |n| {
        if horizontal(n) {
            n.height = *thickness;
        } else {
            n.width = *thickness;
        }
    });
}

fn orientation(
    patch: &mut Patch<FynixHost>,
    orientation: &ControlOrientation,
) {
    let cursor = match orientation {
        ControlOrientation::Horizontal => SystemCursorIcon::NsResize,
        ControlOrientation::Vertical => SystemCursorIcon::EwResize,
    };
    let orientation = *orientation;
    node(patch, move |n| {
        let thickness =
            if horizontal(n) { n.height } else { n.width };
        match orientation {
            ControlOrientation::Horizontal => {
                n.width = percent(100);
                n.height = thickness;
            }
            ControlOrientation::Vertical => {
                n.width = thickness;
                n.height = percent(100);
            }
        }
        n.flex_shrink = 0.0;
    });
    patch.insert(EntityCursor::System(cursor));
}
