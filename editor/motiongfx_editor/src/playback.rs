//! Playback control: play/pause (button + spacebar), scrubbing, and
//! the playhead / time readout.

use bevy::picking::events::{Cancel, Drag, DragEnd, Pointer, Press};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy_motiongfx::prelude::*;

use crate::scene::TimelineContent;
use crate::{EditorState, PIXELS_PER_SECOND};
use bevy::ecs::query::QueryState;
use bevy_motiongfx::prelude::TimelineId;
use motiongfx_editor_ui::reactive::BevyUi;

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
    let mut now_playing = false;
    for mut player in &mut q_players {
        player.is_playing = !player.is_playing;
        player.time_scale = 1.0;
        now_playing |= player.is_playing;
    }
    state.is_playing = now_playing;

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

/// Keep [`EditorState`] tracking the first timeline.
///
/// A binding, not a per-frame system: the predicate only fires when
/// the duration actually changes. It hangs off a UI node purely for
/// lifetime; the write lands on a resource, and the timeline it reads
/// is resolved through a lazily-built query since a predicate only
/// ever holds `&World`.
pub(crate) fn bind_timeline_state(ui: &mut BevyUi) {
    let mut timelines: Option<QueryState<&'static TimelineId>> = None;
    let mut seen: Option<(TimelineId, f32)> = None;

    ui.group().bind_raw(
        move |world, _| {
            let timelines = match &mut timelines {
                Some(query) => query,
                slot => match QueryState::try_new(world) {
                    Some(query) => slot.insert(query),
                    None => return false,
                },
            };
            timelines.update_archetypes(world);
            let current = timelines
                .iter_manual(world)
                .next()
                .copied()
                .map(|id| (id, duration_of(world, id)));
            let changed = seen != current;
            seen = current;
            changed
        },
        |world, _| {
            let Some(id) = first_timeline(world) else {
                return;
            };
            let duration = duration_of(world, id);
            let mut state = world.resource_mut::<EditorState>();
            state.timeline = Some(id);
            state.duration = duration;
        },
    );
}

/// The first timeline in the world, or `None`.
fn first_timeline(world: &mut World) -> Option<TimelineId> {
    world.query::<&TimelineId>().iter(world).next().copied()
}

/// Duration of the timeline's first track, or 0.0 if it is gone.
fn duration_of(world: &World, id: TimelineId) -> f32 {
    world
        .resource::<MotionGfxManager>()
        .get_timeline(&id)
        .and_then(|timeline| {
            timeline.tracks().first().map(|track| track.duration())
        })
        .unwrap_or(0.0)
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
    if state.duration <= 0.0 {
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
            timeline.target_time() <= 0.0
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
