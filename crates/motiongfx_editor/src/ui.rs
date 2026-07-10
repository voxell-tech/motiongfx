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
    Button, ControlOrientation, Slider, SliderOrientation,
    SliderRange, SliderValue, TrackClick,
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

/// Marker for the row name labels in the left column.
#[derive(Component, Default, Clone)]
pub struct RowLabel;

/// One entry in the name column, sized to line up with the action-box
/// row of the same index.
pub fn row_label(text: &str) -> impl Scene {
    let height = Val::Px(super::ROW_HEIGHT);
    let margin = UiRect::bottom(Val::Px(
        super::ROW_STRIDE - super::ROW_HEIGHT,
    ));

    bsn! {
        Node {
            height,
            margin,
            align_items: AlignItems::Center,
        }
        label::<RowLabel>(text)
    }
}

pub fn action_box(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    color: Color,
) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(left),
            top: Val::Px(top),
            width: Val::Px(width),
            height: Val::Px(height),
            border_radius: BorderRadius::all(Val::Px(6.0)),
        }
        BackgroundColor(color)
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
        EntityCursor::System(SystemCursorIcon::Pointer)
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
