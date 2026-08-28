//! Horizontal zoom for the timeline.

use core::time::Duration;

use bevy::input::mouse::MouseScrollUnit;
use bevy::input_focus::InputFocus;
use bevy::picking::events::{Pointer, Scroll};
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::UiGlobalTransform;
use bevy_motiongfx::prelude::MotionGfxManager;

use crate::playback::x_from_cursor;
use crate::ui::timeline::TrackViewport;
use crate::{EditorState, TimelineView};

/// Zoom factor per keypress.
const KEY_STEP: f32 = 1.25;

/// Zoom factor per wheel notch.
const WHEEL_STEP: f32 = 1.1;

/// Zoom in on `=` and out on `-`, about the playhead.
pub(crate) fn zoom_hotkey(
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    q_editable: Query<(), With<EditableText>>,
    q_viewport: Query<&ComputedNode, With<TrackViewport>>,
    state: Res<EditorState>,
    manager: Res<MotionGfxManager>,
    mut view: ResMut<TimelineView>,
) {
    let factor = if keys.just_pressed(KeyCode::Equal) {
        KEY_STEP
    } else if keys.just_pressed(KeyCode::Minus) {
        1.0 / KEY_STEP
    } else {
        return;
    };

    if focus
        .get()
        .is_some_and(|entity| q_editable.contains(entity))
    {
        return;
    }

    let Some(computed) = q_viewport.iter().next() else {
        return;
    };
    let width = computed.size().x * computed.inverse_scale_factor();

    let playhead = state
        .timeline
        .and_then(|id| manager.get_timeline(&id))
        .map(|timeline| timeline.target_time())
        .unwrap_or(Duration::ZERO);
    let playhead_x = view.x_from_time(playhead);
    // A playhead left off screen is pulled back to the middle, since
    // pinning it would keep it off screen at every zoom.
    let anchor_x = if (0.0..=width).contains(&playhead_x) {
        playhead_x
    } else {
        width / 2.0
    };
    view.zoom_to(anchor_x, playhead, factor);
}

/// Zoom on Alt+wheel, pan sideways on Shift+wheel or a
/// horizontal wheel, and scroll the tracks otherwise.
pub(crate) fn on_track_scroll(
    mut scroll: On<Pointer<Scroll>>,
    keys: Res<ButtonInput<KeyCode>>,
    ui_scale: Res<UiScale>,
    mut view: ResMut<TimelineView>,
    mut q_viewport: Query<(
        &ComputedNode,
        &UiGlobalTransform,
        &mut ScrollPosition,
    )>,
) {
    scroll.propagate(false);

    let Ok((computed, transform, mut position)) =
        q_viewport.get_mut(scroll.entity)
    else {
        return;
    };

    let px_per_notch = MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR;
    let delta = Vec2::new(scroll.x, scroll.y)
        * match scroll.unit {
            MouseScrollUnit::Line => px_per_notch,
            MouseScrollUnit::Pixel => 1.0,
        };

    if keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]) {
        let notches = delta.y / px_per_notch;
        let cursor = scroll.pointer_location.position / ui_scale.0;
        let anchor_x = x_from_cursor(cursor, computed, transform);
        let anchor_time = view.time_from_x(anchor_x);
        view.zoom_to(anchor_x, anchor_time, WHEEL_STEP.powf(notches));
        return;
    }

    // Shift sends a vertical wheel sideways, the only pan a mouse
    // without a horizontal one can reach.
    let sideways =
        keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let (pan_x, scroll_y) = if sideways {
        (delta.y, 0.0)
    } else {
        (delta.x, delta.y)
    };

    // Panning goes through the view so the time axis and playhead move
    // with the blocks; `ScrollPosition` only carries y.
    if pan_x != 0.0 {
        view.pan_by(pan_x);
    }

    if scroll_y != 0.0 {
        let inv = computed.inverse_scale_factor();
        let overflow = ((computed.content_size() - computed.size())
            * inv)
            .max(Vec2::ZERO);
        position.y = (position.y - scroll_y).clamp(0.0, overflow.y);
    }
}
