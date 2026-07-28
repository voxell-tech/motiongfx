//! Exact, human-editable [`Duration`] serialization for `#[serde(with =
//! "crate::duration")]`, instead of serde's default `{secs, nanos}` struct.
//!
//! Encodes as whole milliseconds when possible (`Ms(600)`), falling back to
//! nanoseconds only when the duration has sub-millisecond precision
//! (`Ns(1500)`). Deliberately integers, not float seconds: `motiongfx::time`
//! already documents that float durations aren't associative and drift by a
//! few ULPs when accumulated - the same imprecision would round-trip a
//! saved duration to a slightly different value.

use core::time::Duration;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
enum Repr {
    Ms(u64),
    Ns(u64),
}

pub(crate) fn serialize<S: Serializer>(
    duration: &Duration,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let repr = if duration.subsec_nanos().is_multiple_of(1_000_000) {
        Repr::Ms(duration.as_millis() as u64)
    } else {
        Repr::Ns(
            u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX),
        )
    };
    repr.serialize(serializer)
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Duration, D::Error> {
    Ok(match Repr::deserialize(deserializer)? {
        Repr::Ms(ms) => Duration::from_millis(ms),
        Repr::Ns(ns) => Duration::from_nanos(ns),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Wrapper(#[serde(with = "super")] Duration);

    #[test]
    fn whole_milliseconds_encode_as_ms() {
        let json = serde_json::to_string(&Wrapper(
            Duration::from_millis(600),
        ))
        .unwrap();
        assert_eq!(json, r#"{"Ms":600}"#);
    }

    #[test]
    fn sub_millisecond_precision_encodes_as_ns() {
        let json = serde_json::to_string(&Wrapper(
            Duration::from_nanos(1_500),
        ))
        .unwrap();
        assert_eq!(json, r#"{"Ns":1500}"#);
    }

    #[test]
    fn round_trips_exactly() {
        for d in [
            Duration::from_millis(600),
            Duration::from_nanos(1_500),
            Duration::ZERO,
            Duration::from_secs(3),
        ] {
            let json = serde_json::to_string(&Wrapper(d)).unwrap();
            let back: Wrapper = serde_json::from_str(&json).unwrap();
            assert_eq!(back.0, d);
        }
    }
}
