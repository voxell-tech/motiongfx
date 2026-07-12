//! Reusable UI component builders for the editor.
//!
//! Interactive widgets are built from the headless [`bevy::ui_widgets`]
//! behaviors ([`Button`], [`Slider`], ...) and styled with the
//! [`bevy::feathers`] theme so the editor matches the look of Bevy's
//! own tooling.

pub mod dock;
pub mod inspector;

use bevy::feathers::controls::ButtonVariant;
use bevy::feathers::cursor::EntityCursor;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemedText};
use bevy::feathers::tokens;
use bevy::ui::widget::ImageNode;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::{
    Button, ControlOrientation, Slider, SliderOrientation,
    SliderRange, SliderValue, TrackClick,
};
use bevy::window::SystemCursorIcon;

pub const PLAYHEAD_COLOR: Color = Color::srgb(0.95, 0.30, 0.35);

/// A feathers-themed button carrying the headless [`Button`] behavior.
///
/// Emits [`bevy::ui_widgets::Activate`] when clicked or activated via
/// the keyboard; observe that event on the spawned entity.
pub fn themed_button<M>(width: f32, height: f32) -> impl Scene
where
    M: Component + Default + Unpin + Clone,
{
    bsn! {
        M
        Button
        template_value(ButtonVariant::Normal)
        Hovered
        Node {
            width: Val::Px(width),
            height: Val::Px(height),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(Val::Px(6.0)),
        }
        ThemeBackgroundColor(tokens::BUTTON_BG)
        EntityCursor::System(SystemCursorIcon::Pointer)
    }
}

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
/// rebuild without disturbing the playhead thumb (also a content child).
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

/// A clickable collapse toggle for a concurrent group, tagged with the
/// group's stable id. Built on the headless [`Button`] (which consumes
/// the press, so clicking it does not scrub the timeline underneath).
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

/// The playhead line. Doubles as the [`Slider`]'s thumb so the headless
/// slider drag math accounts for its width.
pub fn playhead_line(left: f32) -> impl Scene {
    bsn! {
        bevy::ui_widgets::SliderThumb
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

/// The scrubbable timeline track. The whole track is a horizontal
/// [`Slider`] whose value is the playback time in seconds, so clicking
/// or dragging anywhere on it scrubs. Emits
/// [`bevy::ui_widgets::ValueChange<f32>`].
pub fn scrub_slider(width: f32, duration: f32) -> impl Scene {
    bsn! {
        // `Snap` makes a click jump the value to the cursor; combined
        // with the controlled `SliderValue` writeback in `on_scrub`,
        // dragging then follows the cursor absolutely.
        Slider {
            track_click: TrackClick::Snap,
            orientation: SliderOrientation::Horizontal,
        }
        SliderValue(0.0)
        SliderRange::new(0.0, duration.max(f32::EPSILON))
        Hovered
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

