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

use motiongfx::prelude::*;
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
        Repr::Ms(millis) => ms(millis),
        Repr::Ns(nanos) => ns(nanos),
    })
}

/// The same encoding for `Option<Duration>`, for `#[serde(with =
/// "crate::duration::option")]`. `None` is `null`; pair it with
/// `#[serde(default, skip_serializing_if = "Option::is_none")]` to omit
/// the field entirely.
pub(crate) mod option {
    use super::{
        Deserialize, Deserializer, Duration, Serialize, Serializer,
    };

    #[derive(Serialize, Deserialize)]
    struct Wrapper(#[serde(with = "super")] Duration);

    pub(crate) fn serialize<S: Serializer>(
        duration: &Option<Duration>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        duration.map(Wrapper).serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Duration>, D::Error> {
        Ok(Option::<Wrapper>::deserialize(deserializer)?
            .map(|Wrapper(d)| d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Wrapper(#[serde(with = "super")] Duration);

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct OptionWrapper(
        #[serde(with = "super::option")] Option<Duration>,
    );

    #[test]
    fn option_round_trips_exactly() {
        for d in [Some(ms(600)), None] {
            let json =
                serde_json::to_string(&OptionWrapper(d)).unwrap();
            let back: OptionWrapper =
                serde_json::from_str(&json).unwrap();
            assert_eq!(back.0, d);
        }
    }

    #[test]
    fn whole_milliseconds_encode_as_ms() {
        let json = serde_json::to_string(&Wrapper(ms(600))).unwrap();
        assert_eq!(json, r#"{"Ms":600}"#);
    }

    #[test]
    fn sub_millisecond_precision_encodes_as_ns() {
        let json =
            serde_json::to_string(&Wrapper(ns(1_500))).unwrap();
        assert_eq!(json, r#"{"Ns":1500}"#);
    }

    #[test]
    fn round_trips_exactly() {
        for d in [ms(600), ns(1_500), Duration::ZERO, s(3)] {
            let json = serde_json::to_string(&Wrapper(d)).unwrap();
            let back: Wrapper = serde_json::from_str(&json).unwrap();
            assert_eq!(back.0, d);
        }
    }
}
