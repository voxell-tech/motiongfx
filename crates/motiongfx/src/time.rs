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

/// A half-open time span `[start, end)`.
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Range {
    pub start: Duration,
    pub end: Duration,
}

impl Range {
    /// True when the two [`Range`]s share any point, including a
    /// boundary-only touch.
    pub fn overlap(&self, other: &Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }

    /// The span shared by both [`Range`]s, or `None` when they share
    /// no span of positive length. A boundary-only touch is `None`.
    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        (start < end).then_some(Self { start, end })
    }
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

    #[test]
    fn range_overlap_counts_boundary_touch() {
        let a = Range {
            start: s(0),
            end: s(5),
        };
        let b = Range {
            start: s(3),
            end: s(8),
        };
        let c = Range {
            start: s(6),
            end: s(10),
        };
        let touching = Range {
            start: s(5),
            end: s(5),
        };

        assert!(a.overlap(&b));
        assert!(!a.overlap(&c));
        assert!(a.overlap(&touching));
    }

    #[test]
    fn range_intersect_drops_boundary_touch() {
        let a = Range {
            start: s(0),
            end: s(5),
        };
        let b = Range {
            start: s(3),
            end: s(8),
        };
        let c = Range {
            start: s(6),
            end: s(10),
        };
        let touching = Range {
            start: s(5),
            end: s(8),
        };

        assert_eq!(
            a.intersect(&b),
            Some(Range {
                start: s(3),
                end: s(5)
            }),
        );
        assert_eq!(a.intersect(&c), None);
        assert_eq!(a.intersect(&touching), None);
    }
}
