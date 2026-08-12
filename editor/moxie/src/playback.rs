//! Playback control: play/pause (button + spacebar), scrubbing, and
//! the playhead / time readout.

use core::time::Duration;

use bevy::picking::events::{
    Cancel, Drag, DragEnd, Pointer, Press, Release,
};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy_motiongfx::prelude::*;

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
    mut state: ResMut<EditorState>,
    mut manager: ResMut<MotionGfxManager>,
    mut q_players: Query<&mut RealtimePlayer>,
) {
    // One global target: invert the aggregate, not each player, so
    // mixed states resolve to a single play/pause rather than a swap.
    let should_play = !q_players.iter().any(|p| p.is_playing);
    for mut player in &mut q_players {
        player.is_playing = should_play;
        player.time_scale = 1.0;
    }
    state.is_playing = should_play;

    // Rewind if starting playback from the very end.
    if let Some(timeline_id) = state.timeline
        && should_play
        && let Some(timeline) = manager.get_timeline_mut(&timeline_id)
        && timeline.target_time() >= state.duration
    {
        timeline.set_target_track(0);
        timeline.set_target_time(Duration::ZERO);
    }
}

/// Keep [`EditorState`] tracking the first timeline.
///
/// A system rather than a binding: the write lands on a resource, so
/// it belongs to no node, and hanging it off one purely for lifetime
/// was always a fiction. It writes only when the answer moves, so a
/// change detecting reader still sees one change per change.
pub(crate) fn track_first_timeline(
    timelines: Query<&TimelineId>,
    manager: Res<MotionGfxManager>,
    mut state: ResMut<EditorState>,
) {
    let Some(&id) = timelines.iter().next() else {
        return;
    };

    let duration = manager
        .get_timeline(&id)
        .and_then(|timeline| {
            timeline.tracks().first().map(|track| track.duration())
        })
        .unwrap_or(Duration::ZERO);

    if state.timeline != Some(id) || state.duration != duration {
        state.timeline = Some(id);
        state.duration = duration;
    }
}

/// Present on the timeline track while a scrub is in progress. A
/// scrub is only ever started by a [`Pointer<Press>`] on the track
/// itself, so drags that began anywhere else can't move the playhead.
#[derive(Component)]
pub(crate) struct Scrubbing;

/// Time under `cursor` for a track laid out at
/// [`PIXELS_PER_SECOND`], clamped to the track's duration.
fn time_at_cursor(
    cursor: Vec2,
    computed: &ComputedNode,
    transform: &UiGlobalTransform,
    duration: Duration,
) -> Duration {
    let inv = computed.inverse_scale_factor();
    let (_scale, _angle, center) =
        transform.to_scale_angle_translation();
    let rect = Rect::from_center_size(
        center.trunc() * inv,
        computed.size() * inv,
    );
    let secs = ((cursor.x - rect.min.x) / PIXELS_PER_SECOND).max(0.0);
    Duration::from_secs_f32(secs).min(duration)
}

/// Move the timeline to `time` and stop playback so the scrub isn't
/// immediately overwritten by the player.
fn scrub_to(
    time: Duration,
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
///
/// `press.entity` is already exactly the entity this observer is
/// registered on - `bevy_picking`'s propagation rewrites a `Pointer`
/// event's own `entity` field to match whichever ancestor's observer
/// is currently running, so there's nothing here to filter on.
pub(crate) fn on_track_press(
    mut press: On<Pointer<Press>>,
    state: Res<EditorState>,
    ui_scale: Res<UiScale>,
    q_track: Query<(&ComputedNode, &UiGlobalTransform)>,
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
        With<Scrubbing>,
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

/// End a scrub when a drag finishes off the track.
pub(crate) fn on_track_release(
    release: On<Pointer<DragEnd>>,
    mut commands: Commands,
) {
    commands.entity(release.entity).remove::<Scrubbing>();
}

/// End a scrub on a plain press/release with no intervening drag.
/// [`Pointer<DragEnd>`] never fires for that gesture, so without this
/// a single click would leave [`Scrubbing`] armed.
pub(crate) fn on_track_click_release(
    release: On<Pointer<Release>>,
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

/// Clear [`RealtimePlayer::is_playing`] once playback reaches the end
/// of the current track.
///
/// [`Timeline::set_target_time`] clamps to the track's duration, so
/// the player would otherwise keep "playing" against the clamp and
/// the button would stay stuck on "Pause".
///
/// [`Timeline::set_target_time`]: bevy_motiongfx::prelude::Timeline::set_target_time
pub(crate) fn stop_at_track_end(
    mut state: ResMut<EditorState>,
    manager: Res<MotionGfxManager>,
    mut q_players: Query<&mut RealtimePlayer>,
) {
    let Some(timeline_id) = state.timeline else {
        return;
    };
    let Some(timeline) = manager.get_timeline(&timeline_id) else {
        return;
    };
    if state.duration == Duration::ZERO {
        return;
    }

    // Playing backwards stops at the start instead.
    let mut now_playing = state.is_playing;
    for mut player in &mut q_players {
        if !player.is_playing {
            continue;
        }
        let at_end = if player.time_scale >= 0.0 {
            timeline.target_time() >= state.duration
        } else {
            timeline.target_time() == Duration::ZERO
        };
        if at_end {
            player.is_playing = false;
            now_playing = false;
        }
    }
    if state.is_playing != now_playing {
        state.is_playing = now_playing;
    }
}
