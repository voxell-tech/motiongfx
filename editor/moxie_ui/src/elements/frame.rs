//! [`Frame`]: the sized, optionally glass-backed container almost
//! every other widget's root node turns out to be.

use bevy::prelude::*;

use crate::glass::Glass;

#[derive(SceneComponent, Default, Clone)]
#[scene(FrameProps)]
pub struct Frame;

pub struct FrameProps {
    pub width: Val,
    pub height: Val,
    pub direction: FlexDirection,
    pub align: AlignItems,
    pub justify: JustifyContent,
    pub padding: UiRect,
    pub gap: Val,
    pub radius: Val,
    /// `None` leaves the background untouched, for a frame that only
    /// wants the layout.
    pub glass: Option<Glass>,
}

impl Default for FrameProps {
    fn default() -> Self {
        Self {
            width: auto(),
            height: auto(),
            direction: FlexDirection::Row,
            align: AlignItems::FlexStart,
            justify: JustifyContent::FlexStart,
            padding: UiRect::ZERO,
            gap: Val::ZERO,
            radius: Val::ZERO,
            glass: None,
        }
    }
}

impl Frame {
    fn scene(props: FrameProps) -> impl Scene {
        let FrameProps {
            width,
            height,
            direction,
            align,
            justify,
            padding,
            gap,
            radius,
            glass,
        } = props;
        bsn! {
            Frame
            Node {
                width,
                height,
                flex_direction: direction,
                align_items: align,
                justify_content: justify,
                padding,
                column_gap: gap,
                row_gap: gap,
                border_radius: BorderRadius::all(radius),
            }
            maybe_glass(glass)
        }
    }
}

/// `bsn!` only accepts `path(args)` call syntax for an embedded scene
/// item, not an arbitrary expression, hence this wrapper around
/// `Option::map`.
fn maybe_glass(glass: Option<Glass>) -> impl Scene {
    glass.map(template_value)
}
