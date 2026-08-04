use bevy::feathers::cursor::EntityCursor;
use bevy::feathers::theme::ThemeBackgroundColor;
use bevy::feathers::tokens;
use bevy::prelude::*;
use bevy::ui_widgets::ControlOrientation;
use bevy::window::SystemCursorIcon;

const DIVIDER_WIDTH: f32 = 6.0;

#[derive(SceneComponent, Default, Clone)]
#[scene(DividerProps)]
pub struct Divider;

pub struct DividerProps {
    pub thickness: Val,
    pub orientation: ControlOrientation,
}

impl Default for DividerProps {
    fn default() -> Self {
        Self {
            thickness: Val::Px(DIVIDER_WIDTH),
            orientation: ControlOrientation::Horizontal,
        }
    }
}

impl Divider {
    pub fn scene(
        DividerProps {
            thickness,
            orientation,
        }: DividerProps,
    ) -> impl Scene {
        let (height, width, cursor_icon) = match orientation {
            ControlOrientation::Horizontal => (
                thickness,
                Val::Percent(100.0),
                SystemCursorIcon::NsResize,
            ),
            ControlOrientation::Vertical => (
                Val::Percent(100.0),
                thickness,
                SystemCursorIcon::EwResize,
            ),
        };
        bsn! {
            Divider
            Node {
                width,
                height,
                flex_shrink: 0.0,
            }
            ThemeBackgroundColor(tokens::PANE_HEADER_DIVIDER)
            EntityCursor::System(cursor_icon)

        }
    }
}
