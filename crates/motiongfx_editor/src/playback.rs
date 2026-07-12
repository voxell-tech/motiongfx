//! Playback control: play/pause (button + spacebar), scrubbing, and the
//! playhead / time readout.

use bevy::prelude::*;
use bevy::ui_widgets::{Activate, SliderValue, ValueChange};
use bevy_motiongfx::prelude::*;

use crate::scene::{
    PlayPauseButton, PlayPauseLabel, Playhead, TimeLabel,
    TimelineContent,
};
use crate::{EditorState, PIXELS_PER_SECOND};

/// Command to flip playback, dispatched from the play/pause button and
/// the spacebar hotkey and handled in one place ([`on_toggle_playback`]).
#[derive(Event)]
pub(crate) struct TogglePlayback;

/// Request a toggle when the play/pause button is activated.
pub(crate) fn on_play_pause(
    activate: On<Activate>,
    mut commands: Commands,
    q_button: Query<(), With<PlayPauseButton>>,
) {
    if q_button.get(activate.entity).is_ok() {
        commands.trigger(TogglePlayback);
    }
}

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

/// Scrub the timeline in response to slider drags.
pub(crate) fn on_scrub(
    change: On<ValueChange<f32>>,
    state: Res<EditorState>,
    mut commands: Commands,
    mut manager: ResMut<MotionGfxManager>,
    mut q_players: Query<&mut RealtimePlayer>,
) {
    let Some(timeline_id) = state.timeline else {
        return;
    };

    // Stop playback so the scrub isn't immediately overwritten.
    for mut player in &mut q_players {
        player.is_playing = false;
    }

    // Write the value back (`SliderValue` is a controlled component):
    // this keeps the slider's drag offset correct for absolute
    // cursor-following.
    commands
        .entity(change.source)
        .insert(SliderValue(change.value));

    if let Some(timeline) = manager.get_timeline_mut(&timeline_id) {
        timeline.set_target_track(0);
        timeline.set_target_time(change.value);
    }
}

/// Move the playhead / slider thumb to the current target time and
/// update the time readout.
pub(crate) fn update_playhead(
    state: Res<EditorState>,
    manager: Res<MotionGfxManager>,
    mut commands: Commands,
    mut q_playhead: Query<&mut Node, With<Playhead>>,
    q_value: Query<(Entity, &SliderValue), With<TimelineContent>>,
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
    // Keep the controlled slider value tracking playback so a grab
    // mid-playback starts its drag from the right offset.
    if let Ok((content, value)) = q_value.single()
        && value.0 != time
    {
        commands.entity(content).insert(SliderValue(time));
    }
    for mut text in &mut q_time_label {
        *text = Text::new(format!("{time:.2}s"));
    }
}

/// Clear [`RealtimePlayer::is_playing`] once playback reaches the end
/// of the current track.
///
/// [`Timeline::set_target_time`] clamps to the track's duration, so the
/// player would otherwise keep "playing" against the clamp and the
/// button would stay stuck on "Pause".
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
