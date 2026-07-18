//! Playback control: play/pause (button + spacebar), scrubbing, and
//! the playhead / time readout.

use bevy::picking::events::{Cancel, Drag, DragEnd, Pointer, Press};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy_motiongfx::prelude::*;

use crate::scene::{
    PlayPauseLabel, Playhead, TimeLabel, TimelineContent,
};
use crate::{EditorState, PIXELS_PER_SECOND};
use bevy_motiongfx::prelude::TimelineId;

/// Command to flip playback, dispatched from the play/pause button
/// and the spacebar hotkey and handled in one place
/// ([`on_toggle_playback`]).
#[derive(Event)]
pub(crate) struct TogglePlayback;

/// Request a toggle when the spacebar is pressed.
pub(crate) fn play_pause_hotkey(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if keys.just_pressed(KeyCode::Space) {
        commands.trigger(TogglePlayback);
    }
}

/// Flip `is_playing` for all players, rewinding to the start first if
/// playback is starting from the end of the track.
pub(crate) fn on_toggle_playback(
    _toggle: On<TogglePlayback>,
    state: Res<EditorState>,
    mut manager: ResMut<MotionGfxManager>,
    mut q_players: Query<&mut RealtimePlayer>,
) {
    for mut player in &mut q_players {
        player.is_playing = !player.is_playing;
        player.time_scale = 1.0;
    }

    // Rewind if starting playback from the very end.
    if let Some(timeline_id) = state.timeline
        && q_players.iter().any(|p| p.is_playing)
        && let Some(timeline) = manager.get_timeline_mut(&timeline_id)
        && timeline.target_time() >= state.duration
    {
        timeline.set_target_track(0);
        timeline.set_target_time(0.0);
    }
}

/// Track the first timeline and keep the track node sized to its
/// duration, so cursor x maps onto the full time range.
pub(crate) fn sync_timeline_state(
    mut state: ResMut<EditorState>,
    manager: Res<MotionGfxManager>,
    q_timelines: Query<&TimelineId>,
    mut q_track: Query<&mut Node, With<TimelineContent>>,
) {
    let Some(id) = q_timelines.iter().next().copied() else {
        return;
    };
    let Some(timeline) = manager.get_timeline(&id) else {
        return;
    };
    let duration =
        timeline.tracks().first().map_or(0.0, |t| t.duration());

    state.timeline = Some(id);
    state.duration = duration;

    let width = Val::Px((duration * PIXELS_PER_SECOND).max(1.0));
    for mut node in &mut q_track {
        if node.width != width {
            node.width = width;
            node.min_width = width;
        }
    }
}

/// Present on the timeline track while a scrub is in progress. A
/// scrub is only ever started by a [`Pointer<Press>`] on the track
/// itself, so drags that began anywhere else can't move the playhead.
#[derive(Component)]
pub(crate) struct Scrubbing;

/// Time (seconds) under `cursor` for a track laid out at
/// [`PIXELS_PER_SECOND`], clamped to the track's duration.
fn time_at_cursor(
    cursor: Vec2,
    computed: &ComputedNode,
    transform: &UiGlobalTransform,
    duration: f32,
) -> f32 {
    let inv = computed.inverse_scale_factor();
    let (_scale, _angle, center) =
        transform.to_scale_angle_translation();
    let rect = Rect::from_center_size(
        center.trunc() * inv,
        computed.size() * inv,
    );
    ((cursor.x - rect.min.x) / PIXELS_PER_SECOND)
        .clamp(0.0, duration.max(0.0))
}

/// Move the timeline to `time` and stop playback so the scrub isn't
/// immediately overwritten by the player.
fn scrub_to(
    time: f32,
    state: &EditorState,
    manager: &mut MotionGfxManager,
    q_players: &mut Query<&mut RealtimePlayer>,
) {
    let Some(timeline_id) = state.timeline else {
        return;
    };
    for mut player in q_players {
        player.is_playing = false;
    }
    if let Some(timeline) = manager.get_timeline_mut(&timeline_id) {
        timeline.set_target_track(0);
        timeline.set_target_time(time);
    }
}

/// Begin a scrub: jump the playhead to the press position and arm
/// [`Scrubbing`] so subsequent drags keep following the cursor.
pub(crate) fn on_track_press(
    mut press: On<Pointer<Press>>,
    state: Res<EditorState>,
    ui_scale: Res<UiScale>,
    q_track: Query<
        (&ComputedNode, &UiGlobalTransform),
        With<TimelineContent>,
    >,
    mut manager: ResMut<MotionGfxManager>,
    mut q_players: Query<&mut RealtimePlayer>,
    mut commands: Commands,
) {
    let track = press.entity;
    let Ok((computed, transform)) = q_track.get(track) else {
        return;
    };
    press.propagate(false);
    commands.entity(track).insert(Scrubbing);

    let cursor = press.pointer_location.position / ui_scale.0;
    let time =
        time_at_cursor(cursor, computed, transform, state.duration);
    scrub_to(time, &state, &mut manager, &mut q_players);
}

/// Continue an armed scrub. Dragging past either end clamps, and a
/// drag that never pressed the track is ignored.
pub(crate) fn on_track_drag(
    mut drag: On<Pointer<Drag>>,
    state: Res<EditorState>,
    ui_scale: Res<UiScale>,
    q_track: Query<
        (&ComputedNode, &UiGlobalTransform),
        (With<TimelineContent>, With<Scrubbing>),
    >,
    mut manager: ResMut<MotionGfxManager>,
    mut q_players: Query<&mut RealtimePlayer>,
) {
    let Ok((computed, transform)) = q_track.get(drag.entity) else {
        return;
    };
    drag.propagate(false);

    let cursor = drag.pointer_location.position / ui_scale.0;
    let time =
        time_at_cursor(cursor, computed, transform, state.duration);
    scrub_to(time, &state, &mut manager, &mut q_players);
}

/// End a scrub on release / drag-end / cancel.
pub(crate) fn on_track_release(
    release: On<Pointer<DragEnd>>,
    mut commands: Commands,
) {
    commands.entity(release.entity).remove::<Scrubbing>();
}

pub(crate) fn on_track_cancel(
    cancel: On<Pointer<Cancel>>,
    mut commands: Commands,
) {
    commands.entity(cancel.entity).remove::<Scrubbing>();
}

/// Move the playhead to the current target time and update the time
/// readout.
pub(crate) fn update_playhead(
    state: Res<EditorState>,
    manager: Res<MotionGfxManager>,
    mut q_playhead: Query<&mut Node, With<Playhead>>,
    mut q_time_label: Query<&mut Text, With<TimeLabel>>,
) {
    let Some(timeline_id) = state.timeline else {
        return;
    };
    let Some(timeline) = manager.get_timeline(&timeline_id) else {
        return;
    };
    let time = timeline.target_time();

    for mut node in &mut q_playhead {
        node.left = Val::Px(time * PIXELS_PER_SECOND);
    }
    for mut text in &mut q_time_label {
        *text = Text::new(format!("{time:.2}s"));
    }
}

/// Clear [`RealtimePlayer::is_playing`] once playback reaches the end
/// of the current track.
///
/// [`Timeline::set_target_time`] clamps to the track's duration, so
/// the player would otherwise keep "playing" against the clamp and
/// the button would stay stuck on "Pause".
///
/// [`Timeline::set_target_time`]: bevy_motiongfx::prelude::Timeline::set_target_time
pub(crate) fn stop_at_track_end(
    state: Res<EditorState>,
    manager: Res<MotionGfxManager>,
    mut q_players: Query<&mut RealtimePlayer>,
) {
    let Some(timeline_id) = state.timeline else {
        return;
    };
    let Some(timeline) = manager.get_timeline(&timeline_id) else {
        return;
    };
    if state.duration <= 0.0 {
        return;
    }

    // Playing backwards stops at the start instead.
    for mut player in &mut q_players {
        if !player.is_playing {
            continue;
        }
        let at_end = if player.time_scale >= 0.0 {
            timeline.target_time() >= state.duration
        } else {
            timeline.target_time() <= 0.0
        };
        if at_end {
            player.is_playing = false;
        }
    }
}

/// Keep the play/pause button label in sync with playback state.
pub(crate) fn update_play_label(
    q_players: Query<&RealtimePlayer>,
    mut q_label: Query<&mut Text, With<PlayPauseLabel>>,
) {
    let Some(player) = q_players.iter().next() else {
        return;
    };
    let Ok(mut text) = q_label.single_mut() else {
        return;
    };
    let want = if player.is_playing { "Pause" } else { "Play" };
    if text.as_str() != want {
        *text = Text::new(want);
    }
}
