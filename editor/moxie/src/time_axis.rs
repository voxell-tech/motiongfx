use core::time::Duration;

use crate::TimelineView;

const MIN_TICK_PX: f32 = 10.0;
/// Minimum spacing between labelled ticks.
const MIN_LABEL_PX: f32 = 48.0;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Tick {
    /// Pixels from the left edge of the view.
    pub(crate) x: f32,
    pub(crate) label: Option<String>,
}

/// The finest spacing that still leaves `min_px` between marks at this
/// scale, off a ladder of one and five per power of ten.
fn tick_step(px_per_second: f32, min_px: f32) -> i64 {
    let px_per_ms = px_per_second / 1000.0;
    // Milliseconds a gap has to cover.
    let target = (min_px / px_per_ms).max(1.0);
    // A zero scale makes `target` infinite, so cap the exponent.
    let exp = target.log10().floor().clamp(0.0, 9.0) as u32;
    let magnitude = 10i64.pow(exp);

    if target <= magnitude as f32 {
        magnitude
    } else if target <= 5.0 * magnitude as f32 {
        5 * magnitude
    } else {
        10 * magnitude
    }
}

/// Decimal places follow the step and not the value to ensure one row never
/// mixes `0.5` with `1`.
fn label(ms: i64, major_ms: i64) -> String {
    let secs = ms as f32 / 1000.0;
    // Enough decimals to tell one mark from the next, and no more.
    let decimals = match major_ms {
        1_000.. => 0,
        100.. => 1,
        10.. => 2,
        _ => 3,
    };

    format!("{secs:.decimals$}")
}

/// Every mark visible across a timeline `width` px wide in order.
pub(crate) fn ticks(view: &TimelineView, width: f32) -> Vec<Tick> {
    let px_per_second = view.px_per_second;
    // A zero scale has no marks to give.
    if !(px_per_second.is_finite() && px_per_second > 0.0) {
        return Vec::new();
    }
    let px_per_ms = px_per_second / 1000.0;

    let minor_ms = tick_step(px_per_second, MIN_TICK_PX);
    let major_ms = tick_step(px_per_second, MIN_LABEL_PX);
    // `major_ms` is always a multiple of `minor_ms`.
    let ticks_per_label = major_ms / minor_ms;
    let offset_ms = view.offset.as_millis() as i64;
    // Pad the range for labels near the edges.
    let from_ms = offset_ms - major_ms;
    let to_ms = offset_ms + (width / px_per_ms) as i64 + major_ms;
    // Find the first and last tick indices in the padded range.
    let first_tick = from_ms.div_euclid(minor_ms).max(0);
    let last_tick = to_ms.div_euclid(minor_ms);

    (first_tick..=last_tick)
        .map(|tick_index| {
            let ms = tick_index * minor_ms;
            Tick {
                x: view.x_from_time(Duration::from_millis(ms as u64)),
                label: (tick_index.rem_euclid(ticks_per_label) == 0)
                    .then(|| label(ms, major_ms)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Expected label spacing at different zoom levels.
    const SCALES: [(f32, i64); 6] = [
        (1.0, 50_000),
        (20.0, 5_000),
        (160.0, 500),
        (640.0, 100),
        (5_000.0, 10),
        (20_000.0, 5),
    ];

    /// A view at `px_per_second`, parked `offset_secs` in.
    fn view(px_per_second: f32, offset_secs: f32) -> TimelineView {
        TimelineView {
            px_per_second,
            offset: Duration::from_secs_f32(offset_secs),
        }
    }

    /// Adjacent labels should always display different values.
    #[test]
    fn neighbours_in_a_row_never_read_alike() {
        for (scale, step) in SCALES {
            assert_eq!(tick_step(scale, MIN_LABEL_PX), step);
            for i in 0..20 {
                let a = label(i * step, step);
                let b = label((i + 1) * step, step);
                assert_ne!(
                    a, b,
                    "scale {scale} (step {step}ms) repeats {a}"
                );
            }
        }
    }

    /// Labels should use the expected number of decimal places.
    #[test]
    fn labels_read_as_expected_at_the_current_scale() {
        let step = tick_step(
            TimelineView::default().px_per_second,
            MIN_LABEL_PX,
        );
        assert_eq!(label(0, step), "0.0");
        assert_eq!(label(500, step), "0.5");
        assert_eq!(label(1_000, step), "1.0");
    }

    /// Ticks should cover the visible range and labels should be evenly spaced.
    #[test]
    fn marks_span_the_range_and_label_every_nth() {
        for (scale, _) in SCALES {
            let marks = ticks(&view(scale, 0.0), 800.0);
            assert!(!marks.is_empty(), "no marks at {scale}");

            // Zero is always the first mark, and always labelled.
            let first = marks.first().unwrap();
            assert_eq!(
                first.x, 0.0,
                "scale {scale} misses the origin"
            );
            assert!(
                first.label.is_some(),
                "scale {scale} origin bare"
            );
            assert!(
                marks.last().unwrap().x >= 800.0,
                "scale {scale} stops short"
            );

            let labelled = marks
                .iter()
                .enumerate()
                .filter(|(_, mark)| mark.label.is_some())
                .map(|(i, _)| i)
                .collect::<Vec<_>>();

            assert!(labelled.len() >= 2, "too few labels at {scale}");
            let stride = labelled[1];
            for pair in labelled.windows(2) {
                assert_eq!(
                    pair[1] - pair[0],
                    stride,
                    "scale {scale} labels are uneven: {labelled:?}"
                );
            }
        }
    }

    /// A panned view is covered edge to edge, same as an unpanned one.
    #[test]
    fn marks_cover_a_panned_view() {
        let width = 800.0;
        for (scale, _) in SCALES {
            // Two screens in, so the pan bites at every scale rather
            // than washing out against the clamp at the coarse end.
            let offset_secs = 2.0 * width / scale;
            let marks = ticks(&view(scale, offset_secs), width);
            assert!(!marks.is_empty(), "no marks at {scale}");

            assert!(
                marks.first().unwrap().x <= 0.0,
                "scale {scale} starts inside the view"
            );
            assert!(
                marks.last().unwrap().x >= width,
                "scale {scale} stops short"
            );
        }
    }
}
