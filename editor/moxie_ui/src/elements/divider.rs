use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::ControlOrientation;
use bevy::window::SystemCursorIcon;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::patch::*;

const DIVIDER_WIDTH: f32 = 6.0;

/// The draggable line between two panes.
#[element]
pub struct Divider {
    #[elem(default = px(DIVIDER_WIDTH), patch = PatchThickness)]
    pub thickness: Val,
    #[elem(default = ::Horizontal, patch = PatchOrientation)]
    pub orientation: ControlOrientation,
    #[elem(default = theme.color.hairline.color, patch = PatchBackground)]
    pub color: Color,
}

/// The axis the line runs along, read back from whichever dimension
/// is the full-length one.
fn horizontal(node: &Node) -> bool {
    node.width == percent(100)
}

field_patch!(PatchThickness, Val, |patch, v| {
    node(patch, |n| {
        if horizontal(n) {
            n.height = *v;
        } else {
            n.width = *v;
        }
    });
});

field_patch!(PatchOrientation, ControlOrientation, |patch, v| {
    let cursor = match v {
        ControlOrientation::Horizontal => SystemCursorIcon::NsResize,
        ControlOrientation::Vertical => SystemCursorIcon::EwResize,
    };
    let orientation = *v;
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
});
