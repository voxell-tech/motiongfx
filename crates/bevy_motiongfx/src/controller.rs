use core::time::Duration;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_time::prelude::*;
use motiongfx::time::IntoDuration;

use crate::MotionGfxSystems;
use crate::manager::{MotionGfxManager, TimelineId};

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                fixed_rate_player_update,
                realtime_player_update,
                passive_player_update,
            )
                .in_set(MotionGfxSystems::Controller),
        );
    }
}

fn realtime_player_update(
    mut motiongfx: ResMut<MotionGfxManager>,
    q_timelines: Query<(&TimelineId, &RealtimePlayer)>,
    time: Res<Time>,
) {
    for (id, player) in
        q_timelines.iter().filter(|(_, p)| p.is_playing)
    {
        if let Some(timeline) = motiongfx.get_timeline_mut(id) {
            timeline
                .advance_secs(player.time_scale * time.delta_secs());
        }
    }
}

fn fixed_rate_player_update(
    mut motiongfx: ResMut<MotionGfxManager>,
    mut q_timelines: Query<(&TimelineId, &mut FixedRatePlayer)>,
) {
    for (id, mut player) in
        q_timelines.iter_mut().filter(|(_, p)| p.is_playing)
    {
        if let Some(timeline) = motiongfx.get_timeline_mut(id) {
            // Each frame we update the timeline according to the fps.
            // Derive the time from the frame counter rather than
            // accumulating, so recorded frame N always lands on the
            // same timestamp.
            player.curr_frame += 1;
            timeline.set_target_time(player.frame_time());
        }
    }
}

fn passive_player_update(
    mut motiongfx: ResMut<MotionGfxManager>,
    passive_players: Query<
        (&TimelineId, &PassivePlayer),
        Changed<PassivePlayer>,
    >,
) {
    for (id, player) in passive_players.iter() {
        if let Some(timeline) = motiongfx.get_timeline_mut(id) {
            // Switch tracks first so that `set_target_time` clamps
            // against the new track's duration, not the stale one.
            timeline.set_target_track(player.track_index);
            timeline.set_target_time(player.time);
        }
    }
}

/// A minimal controller for a [`Timeline`] that increments the target
/// time based on Bevy's [`Time::delta_secs()`].
///
/// [`Timeline`]: motiongfx::timeline::Timeline
#[derive(Component, Debug)]
pub struct RealtimePlayer {
    /// Determines if the timeline is currently playing.
    pub is_playing: bool,
    /// The time scale of the player. Set this to negative
    /// to play backwards.
    pub time_scale: f32,
}

impl RealtimePlayer {
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            is_playing: false,
            time_scale: 1.0,
        }
    }

    /// Builder method for setting [`Self::is_playing`].
    #[inline]
    #[must_use]
    pub const fn with_playing(mut self, playing: bool) -> Self {
        self.is_playing = playing;
        self
    }

    /// Builder method for setting [`Self::time_scale`].
    #[inline]
    #[must_use]
    pub const fn with_time_scale(mut self, time_scale: f32) -> Self {
        self.time_scale = time_scale;
        self
    }

    /// Setter method for setting [`Self::is_playing`].
    #[inline]
    pub const fn set_playing(&mut self, playing: bool) -> &mut Self {
        self.is_playing = playing;
        self
    }

    /// Setter method for setting [`Self::time_scale`].
    #[inline]
    pub const fn set_time_scale(
        &mut self,
        time_scale: f32,
    ) -> &mut Self {
        self.time_scale = time_scale;
        self
    }
}

impl Default for RealtimePlayer {
    fn default() -> Self {
        Self::new()
    }
}

/// A controller for [`Timeline`] that increments the sequence time
/// based on based on a specified fps. This is helpful for scene recording.
///
/// [`Timeline`]: motiongfx::timeline::Timeline
#[derive(Component, Debug)]
pub struct FixedRatePlayer {
    /// Determines how many snapshots per second to take.
    pub fps: u16,
    /// Which frame are we currently at now
    pub curr_frame: u64,
    /// Determines if the timeline is currently playing.
    pub is_playing: bool,
}

impl Default for FixedRatePlayer {
    fn default() -> Self {
        Self::new(30)
    }
}

impl FixedRatePlayer {
    #[inline]
    #[must_use]
    pub const fn new(fps: u16) -> Self {
        Self {
            fps,
            curr_frame: 0,
            is_playing: false,
        }
    }

    /// Builder method for setting [`Self::fps`].
    #[inline]
    #[must_use]
    pub const fn with_fps(mut self, fps: u16) -> Self {
        self.fps = fps;
        self
    }

    /// Calculates the delta seconds based on [`Self::fps`].
    #[inline]
    #[must_use]
    pub const fn delta_secs(&self) -> f32 {
        1.0 / self.fps as f32
    }

    /// The exact timestamp of [`Self::curr_frame`].
    ///
    /// Derived from the frame counter rather than accumulated, so a
    /// given frame index always maps to the same instant no matter how
    /// long the recording runs.
    #[inline]
    #[must_use]
    pub const fn frame_time(&self) -> Duration {
        let fps = if self.fps == 0 { 1 } else { self.fps as u64 };

        Duration::from_nanos(
            1_000_000_000u64.saturating_mul(self.curr_frame) / fps,
        )
    }

    /// Setter method for setting [`Self::is_playing`].
    #[inline]
    pub const fn set_playing(
        &mut self,
        is_playing: bool,
    ) -> &mut Self {
        self.is_playing = is_playing;
        self
    }
}
#[derive(Default, Component)]
pub struct PassivePlayer {
    time: Duration,
    track_index: usize,
}

impl PassivePlayer {
    #[inline]
    pub fn set_time(&mut self, time: impl IntoDuration) {
        self.time = time.into_duration();
    }

    #[inline]
    pub fn set_track_index(&mut self, track_index: usize) {
        self.track_index = track_index;
    }

    pub fn get_time(&self) -> Duration {
        self.time
    }

    pub fn get_track(&self) -> usize {
        self.track_index
    }
}
