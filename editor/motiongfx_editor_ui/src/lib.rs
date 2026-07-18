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

// Inherent to Bevy ECS: systems take many params and query tuples.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

pub mod dock;
pub mod glass;
pub mod inspector;
pub mod theme;

use bevy::feathers::cursor::EntityCursor;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemedText};
use bevy::feathers::tokens;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::widget::ImageNode;
use bevy::ui_widgets::{Button, ControlOrientation};
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

/// Marker for a generated clip box, so the boxes can be despawned on
/// rebuild without disturbing the playhead thumb (also a content
/// child).
#[derive(Component, Default, Clone)]
pub struct ActionBox;

/// Marker for the clip box's inline label text.
#[derive(Component, Default, Clone)]
pub struct ClipLabel;

/// A single action clip: a colored, rounded box positioned absolutely
/// within the timeline content, with its field label inside.
pub fn clip_box(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    color: Color,
    text: &str,
) -> impl Scene {
    bsn! {
        ActionBox
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(left),
            top: Val::Px(top),
            width: Val::Px(width),
            height: Val::Px(height),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(6.0)),
            overflow: Overflow::clip(),
        }
        ZIndex(1)
        BackgroundColor(color)
        Children [
            (
                ClipLabel
                Text({text})
                TextFont {
                    font_size: FontSize::Px(11.0)
                }
                TextColor(Color::srgb(0.1, 0.1, 0.12))
            )
        ]
    }
}

/// Marker for a generated group container, for teardown on rebuild.
#[derive(Component, Default, Clone)]
pub struct GroupBox;

/// A container drawn behind the boxes of a concurrent group
/// ([`Combinator::All`] / `Any` / `Flow`), grouping the rows that run
/// together.
///
/// [`Combinator::All`]: bevy_motiongfx::prelude::Combinator
pub fn group_box(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
) -> impl Scene {
    let accent = Color::srgb(0.55, 0.60, 0.70);
    let fill = accent.with_alpha(0.08);
    bsn! {
        GroupBox
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(left),
            top: Val::Px(top),
            width: Val::Px(width),
            height: Val::Px(height),
            border: UiRect::all(Val::Px(1.5)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
        }
        ZIndex(0)
        BorderColor::all(accent)
        BackgroundColor(fill)
    }
}

/// A clickable collapse toggle for a concurrent group, tagged with
/// the group's stable id. Built on the headless [`Button`] (which
/// consumes the press, so clicking it does not scrub the timeline
/// underneath).
#[derive(Component, Clone, Default)]
pub struct GroupToggle(pub usize);

#[expect(clippy::too_many_arguments)]
pub fn group_toggle(
    gid: usize,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    icon_path: &'static str,
    text: &str,
    bg: Color,
) -> impl Scene {
    bsn! {
        GroupToggle(gid)
        Button
        Hovered
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(left),
            top: Val::Px(top),
            width: Val::Px(width),
            height: Val::Px(height),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(4.0),
            padding: UiRect::horizontal(Val::Px(4.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            overflow: Overflow::clip(),
        }
        ZIndex(5)
        BackgroundColor(bg)
        EntityCursor::System(SystemCursorIcon::Pointer)
        Children [
            (
                Node { height: Val::Px(12.0) }
                ImageNode { image: icon_path }
            ),
            (
                Text({text})
                TextFont { font_size: FontSize::Px(11.0) }
                TextColor(Color::srgb(0.9, 0.9, 0.92))
            ),
        ]
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
