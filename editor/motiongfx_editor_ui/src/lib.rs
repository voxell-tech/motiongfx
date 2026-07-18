//! Reusable `bevy_ui` building blocks for the MotionGfx editor:
//! widget builders here, plus [`dock`] (docking engine), [`glass`]
//! (frosted-glass material), [`inspector`] (reflect inspector) and
//! [`theme`].
//!
//! **UI-only**: no `bevy_motiongfx` or editor domain deps, so the
//! docking system is reusable standalone.
//!
//! Widgets build on the headless [`bevy::ui_widgets`] behaviors and
//! the [`bevy::feathers`] theme.

#![allow(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "Inherent to Bevy ECS: systems take many params and query tuples."
)]

pub mod dock;
pub mod glass;
pub mod inspector;
pub mod theme;

use bevy::feathers::cursor::EntityCursor;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemedText};
use bevy::feathers::tokens;
use bevy::prelude::*;
use bevy::ui_widgets::ControlOrientation;
use bevy::window::SystemCursorIcon;

pub const PLAYHEAD_COLOR: Color = Color::srgb(0.95, 0.30, 0.35);

/// A theme-inheriting text label.
pub fn label<M: Component + Default + Unpin + Clone>(
    text: &str,
) -> impl Scene {
    bsn! {
        M
        Text({text})
        ThemedText
        TextFont {
            font_size: FontSize::Px(13.0)
        }

    }
}

/// The playhead line, positioned by the editor's playhead system.
pub fn playhead_line(left: f32) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            left: Val::Px(left),
            width: Val::Px(2.0),
        }
        ZIndex(10)
        BackgroundColor(PLAYHEAD_COLOR)
    }
}

/// The scrubbable timeline track: a plain node sized to the track's
/// duration (`PIXELS_PER_SECOND` per second), so a clip at time `t`
/// sits at `t * PIXELS_PER_SECOND` from its left edge.
///
/// Scrubbing is driven by pointer observers on this node (see
/// the editor's track pointer observers) rather than a
/// headless `Slider`: a scrub can only *begin* from a press that
/// actually lands inside the track, so it can't be started from
/// elsewhere in the window.
pub fn timeline_track(width: f32) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Relative,
            width: Val::Px(width),
            min_width: Val::Px(width),
            height: Val::Percent(100.0),
        }
    }
}

pub const DIVIDER_WIDTH: f32 = 6.0;

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
