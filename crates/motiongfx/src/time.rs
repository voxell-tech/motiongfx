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

/// Whole centiseconds (hundredths of a second) as a [`Duration`].
#[inline]
#[must_use]
pub const fn cs(centis: u64) -> Duration {
    Duration::from_millis(centis.saturating_mul(10))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_helpers_agree_with_duration_constructors() {
        assert_eq!(s(2), ms(2_000));
        assert_eq!(cs(150), ms(1_500));
        assert_eq!(ms(1), ns(1_000_000));
        assert_eq!(cs(u64::MAX), Duration::from_millis(u64::MAX));
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
}
