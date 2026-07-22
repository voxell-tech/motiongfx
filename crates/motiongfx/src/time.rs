//! Time conversion helpers.
//!
//! Timing is stored as [`Duration`] so that the durations accumulated by
//! the track combinators and the clip offsets accumulated when delaying a
//! [`Sequence`] can never disagree. Float seconds are not associative, so
//! the two used to drift apart by a few ULPs, tripping the non-overlap
//! assertion and leaving the playhead clamp short of the final clip's end.
//!
//! [`IntoDuration`] keeps seconds as the authoring unit.
//!
//! [`Sequence`]: crate::sequence::Sequence

use core::time::Duration;

/// Conversion into a [`Duration`] for the timing arguments of the
/// authoring API, so that `play(0.6)` and
/// `play(Duration::from_millis(600))` are both accepted.
///
/// Unrepresentable seconds saturate rather than panic: negative and NaN
/// become [`Duration::ZERO`], overflow becomes [`Duration::MAX`]. This
/// keeps `set_target_time(f32::MAX)` ("seek to the end") working.
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

        // Positive and non-NaN, so the only remaining failure is
        // overflow. `from_secs_f64` would panic on it.
        Duration::try_from_secs_f64(self).unwrap_or(Duration::MAX)
    }
}

/// Whole seconds as a [`Duration`].
#[inline]
#[must_use]
pub const fn s(secs: u64) -> Duration {
    Duration::from_secs(secs)
}

/// Whole milliseconds as a [`Duration`].
#[inline]
#[must_use]
pub const fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// Whole nanoseconds as a [`Duration`].
#[inline]
#[must_use]
pub const fn ns(nanos: u64) -> Duration {
    Duration::from_nanos(nanos)
}

/// Offsets `time` by `delta` seconds, saturating at [`Duration::ZERO`].
///
/// [`Duration`] has no signed representation, so stepping a playhead
/// backwards has to go through this rather than a plain `+`.
///
/// Only as precise as `delta` itself: `0.05f32` is really
/// `0.05000000074505806`, so it lands a nanosecond off. That is inherent
/// to a float clock, not the drift this module prevents. Clip and track
/// boundaries stay exact, so the error does not accumulate.
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
    fn unit_helpers_agree_with_duration_constructors() {
        assert_eq!(s(2), ms(2_000));
        assert_eq!(ms(1), ns(1_000_000));
    }

    /// Floats here are the subject under test, not a time literal.
    #[test]
    fn unrepresentable_seconds_saturate_instead_of_panicking() {
        assert_eq!((-1.0f32).into_duration(), Duration::ZERO);
        assert_eq!(f32::NAN.into_duration(), Duration::ZERO);
        assert_eq!(f32::NEG_INFINITY.into_duration(), Duration::ZERO);
        assert_eq!(f32::MAX.into_duration(), Duration::MAX);
        assert_eq!(f32::INFINITY.into_duration(), Duration::MAX);
    }

    /// `delta` is `f32` seconds, so exact equality is not assertable.
    #[track_caller]
    fn assert_near(actual: Duration, expected: Duration) {
        let diff = actual.abs_diff(expected);

        assert!(
            diff < ns(1_000),
            "{actual:?} is not within 1us of {expected:?}",
        );
    }

    #[test]
    fn offset_secs_steps_both_ways() {
        assert_near(offset_secs(ms(100), -0.05), ms(50));
        assert_near(offset_secs(ms(100), 0.05), ms(150));
    }

    #[test]
    fn offset_secs_saturates_at_zero() {
        assert_eq!(offset_secs(ms(100), -10.0), Duration::ZERO);
        assert_eq!(offset_secs(Duration::ZERO, -1.0), Duration::ZERO);
    }
}
