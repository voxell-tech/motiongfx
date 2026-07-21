//! Time conversion helpers.
//!
//! Motiongfx stores all timing as [`Duration`] so that the durations
//! accumulated by the track combinators and the clip offsets accumulated
//! when delaying a [`Sequence`] can never disagree. Float seconds are not
//! associative, so the two accumulations used to drift apart by a few
//! ULPs, which tripped the non-overlap assertion when extending a
//! sequence and left the playhead clamp just short of the final clip's
//! end.
//!
//! Authoring in seconds is still the ergonomic default, hence
//! [`IntoDuration`].
//!
//! [`Sequence`]: crate::sequence::Sequence

use core::time::Duration;

/// Conversion into a [`Duration`] for the timing arguments of the
/// authoring API.
///
/// Implemented for [`Duration`] itself as well as `f32`/`f64` seconds,
/// so both of these are accepted:
///
/// ```ignore
/// action.play(0.6);
/// action.play(Duration::from_millis(600));
/// ```
///
/// Seconds that a [`Duration`] cannot represent saturate rather than
/// panic: negative and non-finite values become [`Duration::ZERO`],
/// and values past [`Duration::MAX`] become [`Duration::MAX`]. This
/// keeps idioms like `set_target_time(f32::MAX)` ("seek to the end")
/// working.
pub trait IntoDuration {
    fn into_duration(self) -> Duration;
}

impl IntoDuration for Duration {
    #[inline]
    fn into_duration(self) -> Duration {
        self
    }
}

impl IntoDuration for f32 {
    #[inline]
    fn into_duration(self) -> Duration {
        (self as f64).into_duration()
    }
}

impl IntoDuration for f64 {
    #[inline]
    fn into_duration(self) -> Duration {
        if self.is_nan() || self <= 0.0 {
            return Duration::ZERO;
        }

        // Positive and non-NaN at this point, so the only remaining
        // failure is overflow (which includes `INFINITY`).
        // `from_secs_f64` would panic on those instead.
        Duration::try_from_secs_f64(self).unwrap_or(Duration::MAX)
    }
}

/// Offsets `time` by `delta` seconds, saturating at [`Duration::ZERO`].
///
/// [`Duration`] has no signed representation, so stepping a playhead
/// backwards has to go through this rather than a plain `+`.
///
/// The offset is only as precise as `delta` itself: `0.05f32` is really
/// `0.05000000074505806`, so it lands a nanosecond off. That is
/// inherent to a float-seconds clock and is not the drift this module
/// exists to prevent — clip and track boundaries are exact
/// [`Duration`]s, so an imprecise step still resolves against them
/// consistently instead of accumulating error into the timeline.
#[inline]
pub fn offset_secs(time: Duration, delta: f32) -> Duration {
    if delta >= 0.0 {
        time.saturating_add(delta.into_duration())
    } else {
        time.saturating_sub((-delta).into_duration())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrepresentable_seconds_saturate_instead_of_panicking() {
        assert_eq!((-1.0f32).into_duration(), Duration::ZERO);
        assert_eq!(f32::NAN.into_duration(), Duration::ZERO);
        assert_eq!(f32::NEG_INFINITY.into_duration(), Duration::ZERO);
        // "Seek to the end" idiom: must clamp, not overflow.
        assert_eq!(f32::MAX.into_duration(), Duration::MAX);
        assert_eq!(f32::INFINITY.into_duration(), Duration::MAX);
    }

    /// `delta` is `f32` seconds, so the result is only accurate to
    /// within the rounding of that `f32` — a nanosecond or so. Exact
    /// equality is deliberately not asserted here.
    #[track_caller]
    fn assert_near(actual: Duration, expected: Duration) {
        let diff = actual.abs_diff(expected);

        assert!(
            diff < Duration::from_micros(1),
            "{actual:?} is not within 1us of {expected:?}",
        );
    }

    #[test]
    fn offset_secs_steps_both_ways() {
        let time = Duration::from_millis(100);

        assert_near(
            offset_secs(time, -0.05),
            Duration::from_millis(50),
        );
        assert_near(
            offset_secs(time, 0.05),
            Duration::from_millis(150),
        );
    }

    #[test]
    fn offset_secs_saturates_at_zero() {
        let time = Duration::from_millis(100);

        // Stepping further back than the playhead has to give must
        // saturate rather than wrap or panic.
        assert_eq!(offset_secs(time, -10.0), Duration::ZERO);
        assert_eq!(offset_secs(Duration::ZERO, -1.0), Duration::ZERO);
    }
}
