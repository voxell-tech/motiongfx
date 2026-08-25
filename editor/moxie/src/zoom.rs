//! Horizontal zoom for the timeline.

use bevy::input::mouse::MouseScrollUnit;
use bevy::input_focus::InputFocus;
use bevy::picking::events::{Pointer, Scroll};
use bevy::prelude::*;
use bevy::text::EditableText;

use crate::TimelineView;

/// Zoom factor per keypress.
const KEY_STEP: f32 = 1.25;

/// Zoom factor per wheel notch.
const WHEEL_STEP: f32 = 1.1;

/// Zoom in on `=` and out on `-`.
pub(crate) fn zoom_hotkey(
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    q_editable: Query<(), With<EditableText>>,
    mut view: ResMut<TimelineView>,
) {
    if focus
        .get()
        .is_some_and(|entity| q_editable.contains(entity))
    {
        return;
    }

    if keys.just_pressed(KeyCode::Equal) {
        view.zoom_by(KEY_STEP);
    } else if keys.just_pressed(KeyCode::Minus) {
        view.zoom_by(1.0 / KEY_STEP);
    }
}

/// Zoom on Alt+wheel, and scroll the tracks otherwise.
pub(crate) fn on_track_scroll(
    mut scroll: On<Pointer<Scroll>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut view: ResMut<TimelineView>,
    mut q_viewport: Query<(&ComputedNode, &mut ScrollPosition)>,
) {
    scroll.propagate(false);

    let delta = match scroll.unit {
        MouseScrollUnit::Line => {
            scroll.y * MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR
        }
        MouseScrollUnit::Pixel => scroll.y,
    };
    if keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]) {
        let notches =
            delta / MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR;
        view.zoom_by(WHEEL_STEP.powf(notches));
        return;
    }

    let Ok((computed, mut position)) =
        q_viewport.get_mut(scroll.entity)
    else {
        return;
    };
    let inv = computed.inverse_scale_factor();
    let overflow = ((computed.content_size() - computed.size())
        * inv)
        .max(Vec2::ZERO);
    // Only y. `offset` pans instead, so the time axis and playhead move
    // with the blocks; x stays `Scroll` to clip them at the edge.
    position.y = (position.y - delta).clamp(0.0, overflow.y);
}
