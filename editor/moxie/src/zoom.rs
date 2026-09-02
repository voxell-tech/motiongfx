//! Horizontal zoom for the timeline.

use bevy::input::mouse::MouseScrollUnit;
use bevy::picking::events::{Pointer, Scroll};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use crate::playback::x_from_cursor;
use crate::ui::timeline::TrackViewport;
use crate::{EditorState, TimelineView};

/// Zoom factor per wheel notch.
const WHEEL_STEP: f32 = 1.1;

/// Command to fit the animation to the panel, dispatched from the fit
/// button and handled in [`on_fit_timeline`].
#[derive(Event)]
pub(crate) struct FitTimeline;

/// Scale the view so the animation spans the track viewport.
pub(crate) fn on_fit_timeline(
    _fit: On<FitTimeline>,
    q_viewport: Query<&ComputedNode, With<TrackViewport>>,
    state: Res<EditorState>,
    mut view: ResMut<TimelineView>,
) {
    let Some(computed) = q_viewport.iter().next() else {
        return;
    };
    let width = computed.size().x * computed.inverse_scale_factor();
    view.fit(width, state.duration);
}

/// Zoom on Alt+wheel, pan sideways on Shift+wheel or a
/// horizontal wheel, and scroll the tracks otherwise.
pub(crate) fn on_track_scroll(
    mut scroll: On<Pointer<Scroll>>,
    keys: Res<ButtonInput<KeyCode>>,
    ui_scale: Res<UiScale>,
    mut view: ResMut<TimelineView>,
    mut q_viewport: Query<
        (&ComputedNode, &UiGlobalTransform, &mut ScrollPosition),
        With<TrackViewport>,
    >,
) {
    scroll.propagate(false);

    let Some((computed, transform, mut position)) =
        q_viewport.iter_mut().next()
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

    // Normalize Shift+wheel into horizontal scrolling across platforms.
    let sideways =
        keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let (pan_x, scroll_y) = if sideways {
        (if delta.x != 0.0 { delta.x } else { delta.y }, 0.0)
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
