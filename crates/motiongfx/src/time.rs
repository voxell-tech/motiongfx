//! Time conversion helpers.
//!
//! Timing is stored as [`Duration`] so that the durations accumulated by
//! the track combinators and the clip offsets accumulated when delaying a
//! [`Sequence`] can never disagree. Float seconds are not associative, so
//! the two used to drift apart by a few ULPs, tripping the non-overlap
//! assertion and leaving the playhead clamp short of the final clip's end.
//!
//! [`Sequence`]: crate::sequence::Sequence

use core::time::Duration;

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
}
