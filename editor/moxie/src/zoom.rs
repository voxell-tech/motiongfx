//! Horizontal zoom for the timeline.

use bevy::input::mouse::MouseScrollUnit;
use bevy::picking::events::{Pointer, Scroll};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use crate::TimelineView;
use crate::playback::x_from_cursor;

/// Zoom factor per wheel notch.
const WHEEL_STEP: f32 = 1.1;

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
