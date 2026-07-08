//! Reusable UI component builders for the editor.
//!
//! Interactive widgets are built from the headless [`bevy::ui_widgets`]
//! behaviors ([`Button`], [`Slider`], ...) and styled with the
//! [`bevy::feathers`] theme so the editor matches the look of Bevy's
//! own tooling.

use bevy::feathers::controls::ButtonVariant;
use bevy::feathers::cursor::EntityCursor;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemedText};
use bevy::feathers::tokens;
use bevy::picking::events::{Drag, Pointer};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::{
    Button, Slider, SliderOrientation, SliderRange, SliderValue,
    TrackClick,
};
use bevy::window::SystemCursorIcon;

/// Background color of a single action box. Kept as a literal palette
/// (rather than a theme token) so the different rows read as distinct
/// clips regardless of theme.
pub const ACTION_COLORS: [Color; 6] = [
    Color::srgb(0.30, 0.62, 0.92),
    Color::srgb(0.42, 0.78, 0.52),
    Color::srgb(0.92, 0.66, 0.30),
    Color::srgb(0.78, 0.44, 0.86),
    Color::srgb(0.90, 0.44, 0.50),
    Color::srgb(0.36, 0.78, 0.80),
];

pub const PLAYHEAD_COLOR: Color = Color::srgb(0.95, 0.30, 0.35);

pub fn row_color(row: usize) -> Color {
    ACTION_COLORS[row % ACTION_COLORS.len()]
}

/// A feathers-themed button carrying the headless [`Button`] behavior.
///
/// Emits [`bevy::ui_widgets::Activate`] when clicked or activated via
/// the keyboard; observe that event on the spawned entity.
pub fn themed_button<M: Component>(
    marker: M,
    width: f32,
    height: f32,
) -> impl Bundle {
    (
        marker,
        Button,
        ButtonVariant::Normal,
        Hovered::default(),
        Node {
            width: Val::Px(width),
            height: Val::Px(height),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        ThemeBackgroundColor(tokens::BUTTON_BG),
        EntityCursor::System(SystemCursorIcon::Pointer),
    )
}

/// A theme-inheriting text label.
pub fn label<M: Component>(marker: M, text: &str) -> impl Bundle {
    (
        marker,
        Text::new(text),
        ThemedText,
        TextFont::from_font_size(13.0),
    )
}

pub fn action_box(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    color: Color,
) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(left),
            top: Val::Px(top),
            width: Val::Px(width),
            height: Val::Px(height),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(color),
    )
}

/// The playhead line. Doubles as the [`Slider`]'s thumb so the headless
/// slider drag math accounts for its width.
pub fn playhead_line(left: f32) -> impl Bundle {
    (
        bevy::ui_widgets::SliderThumb,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            left: Val::Px(left),
            width: Val::Px(2.0),
            ..default()
        },
        ZIndex(10),
        BackgroundColor(PLAYHEAD_COLOR),
    )
}

/// The scrubbable timeline track. The whole track is a horizontal
/// [`Slider`] whose value is the playback time in seconds, so clicking
/// or dragging anywhere on it scrubs. Emits
/// [`bevy::ui_widgets::ValueChange<f32>`].
pub fn scrub_slider(width: f32, duration: f32) -> impl Bundle {
    (
        // `Snap` makes a click jump the value to the cursor; combined
        // with the controlled `SliderValue` writeback in `on_scrub`,
        // dragging then follows the cursor absolutely.
        Slider {
            track_click: TrackClick::Snap,
            orientation: SliderOrientation::Horizontal,
        },
        SliderValue(0.0),
        SliderRange::new(0.0, duration.max(f32::EPSILON)),
        Hovered::default(),
        EntityCursor::System(SystemCursorIcon::Pointer),
        Node {
            position_type: PositionType::Relative,
            width: Val::Px(width),
            min_width: Val::Px(width),
            height: Val::Percent(100.0),
            ..default()
        },
    )
}

pub const DIVIDER_WIDTH: f32 = 4.0;
pub const PANEL_HANDLE_HEIGHT: f32 = 6.0;

#[derive(Component)]
pub struct ResizeDivider;

#[derive(Component)]
pub struct PanelResizeHandle;

/// A full-width grab handle along the top edge of the panel. Dragging
/// it resizes the (bottom-anchored) panel vertically.
pub fn panel_resize_handle() -> impl Bundle {
    (
        PanelResizeHandle,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(PANEL_HANDLE_HEIGHT),
            flex_shrink: 0.0,
            ..default()
        },
        ThemeBackgroundColor(tokens::PANE_HEADER_DIVIDER),
        EntityCursor::System(SystemCursorIcon::NsResize),
    )
}

/// Drag handler for the panel's top-edge resize handle.
pub fn on_panel_resize(
    drag: On<Pointer<Drag>>,
    q_panel: Query<Entity, With<super::EditorPanel>>,
    q_window: Query<&Window>,
    mut q_nodes: Query<&mut Node>,
) {
    let delta = drag.delta.y;
    if delta == 0.0 {
        return;
    }
    let Ok(panel) = q_panel.single() else {
        return;
    };
    // Dragging up (negative delta) should grow the panel.
    let max = q_window
        .iter()
        .next()
        .map(|w| w.height() - super::CONTROL_BAR_HEIGHT)
        .unwrap_or(super::PANEL_MAX_HEIGHT)
        .min(super::PANEL_MAX_HEIGHT);
    let Ok(mut panel_node) = q_nodes.get_mut(panel) else {
        return;
    };
    if let Val::Px(h) = panel_node.height {
        let new_h = (h - delta).clamp(super::PANEL_MIN_HEIGHT, max);
        panel_node.height = Val::Px(new_h);
    }
}

pub fn resize_divider() -> impl Bundle {
    (
        ResizeDivider,
        Node {
            width: Val::Px(DIVIDER_WIDTH),
            height: Val::Percent(100.0),
            flex_shrink: 0.0,
            ..default()
        },
        ThemeBackgroundColor(tokens::PANE_HEADER_DIVIDER),
        EntityCursor::System(SystemCursorIcon::EwResize),
    )
}

/// Drag handler for the name-panel / track resize divider.
pub fn on_divider_drag(
    drag: On<Pointer<Drag>>,
    q_name_panel: Query<Entity, With<super::NamePanel>>,
    mut q_nodes: Query<&mut Node>,
) {
    let delta = drag.delta.x;
    if delta == 0.0 {
        return;
    }
    let Ok(name_panel) = q_name_panel.single() else {
        return;
    };
    let Ok(mut panel_node) = q_nodes.get_mut(name_panel) else {
        return;
    };
    if let Val::Px(w) = panel_node.width {
        let new_w = (w + delta)
            .clamp(super::NAME_PANEL_MIN, super::NAME_PANEL_MAX);
        panel_node.width = Val::Px(new_w);
    }
}
