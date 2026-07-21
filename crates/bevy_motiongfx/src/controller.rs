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
    /// A given frame index always maps to the same instant no matter
    /// how long the recording runs.
    ///
    /// Returns [`Duration::ZERO`] when [`Self::fps`] is zero.
    #[inline]
    #[must_use]
    pub fn frame_time(&self) -> Duration {
        if self.fps == 0 {
            return Duration::ZERO;
        }

        Duration::from_secs(self.curr_frame) / self.fps as u32
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

#[cfg(test)]
mod tests {
    use super::*;

    fn at(fps: u16, frame: u64) -> Duration {
        FixedRatePlayer {
            fps,
            curr_frame: frame,
            is_playing: false,
        }
        .frame_time()
    }

    #[test]
    fn frame_time_is_exact_where_the_rate_divides() {
        assert_eq!(at(30, 0), Duration::ZERO);
        assert_eq!(at(30, 30), Duration::from_secs(1));
        assert_eq!(at(30, 3), Duration::from_millis(100));
        assert_eq!(at(25, 1), Duration::from_millis(40));
        assert_eq!(at(60, 90), Duration::from_millis(1500));
    }

    /// The whole point of deriving from the counter: no accumulated
    /// error, so a long render lands on exact second boundaries.
    #[test]
    fn frame_time_does_not_drift_over_a_long_render() {
        // 30 minutes at 30fps.
        assert_eq!(at(30, 54_000), Duration::from_secs(1800));
        // Well past where the old nanosecond-based form overflowed.
        assert_eq!(
            at(30, 30_000_000_000),
            Duration::from_secs(1_000_000_000)
        );
    }

    /// 1/3s is not representable in nanoseconds, so consecutive
    /// frames differ by a nanosecond. The error must not compound.
    #[test]
    fn frame_time_stays_within_a_nanosecond_of_ideal() {
        for frame in [1u64, 2, 1_000, 999_999] {
            let ideal = Duration::from_nanos(
                (frame as u128 * 1_000_000_000 / 3) as u64,
            );

            assert_eq!(at(3, frame).abs_diff(ideal), Duration::ZERO);
        }
    }

    /// A rate of zero frames per second advances no time, rather than
    /// dividing by zero or standing in some invented rate.
    #[test]
    fn zero_fps_yields_zero() {
        assert_eq!(at(0, 0), Duration::ZERO);
        assert_eq!(at(0, 5), Duration::ZERO);
    }
}
