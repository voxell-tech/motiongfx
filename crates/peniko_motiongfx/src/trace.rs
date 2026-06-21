use peniko::kurbo::{BezPath, CubicBez, Line, ParamCurve, Point, QuadBez};

pub type LineTracer = Tracer<Line>;
pub type QuadTracer = Tracer<QuadBez>;
pub type CubicTracer = Tracer<CubicBez>;
pub type PathTracer = Tracer<BezPath>;

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Tracer<T: Trace> {
    /// The original full path.
    pub path: T,
    /// Normalized start of the visible range (0..1).
    pub t_start: f32,
    /// Normalized end of the visible range (0..1).
    pub t_end: f32,
}

impl<T: Trace> Tracer<T> {
    pub fn trace(&self) -> T {
        self.path.trace_range(self.t_start, self.t_end)
    }
}

pub trait Trace {
    /// Returns the slice of `self` from `t_start` to `t_end` (both in 0..1).
    fn trace_range(&self, t_start: f32, t_end: f32) -> Self;

    /// Returns the prefix of `self` from 0 to `t`.
    fn trace(&self, t: f32) -> Self
    where
        Self: Sized,
    {
        self.trace_range(0.0, t)
    }
}

impl Trace for Line {
    #[inline]
    fn trace_range(&self, t_start: f32, t_end: f32) -> Self {
        let t_start = t_start.clamp(0.0, 1.0) as f64;
        let t_end = t_end.clamp(0.0, 1.0) as f64;
        self.subsegment(t_start..t_end)
    }
}

impl Trace for QuadBez {
    #[inline]
    fn trace_range(&self, t_start: f32, t_end: f32) -> Self {
        let t_start = t_start.clamp(0.0, 1.0) as f64;
        let t_end = t_end.clamp(0.0, 1.0) as f64;
        self.subsegment(t_start..t_end)
    }
}

impl Trace for CubicBez {
    #[inline]
    fn trace_range(&self, t_start: f32, t_end: f32) -> Self {
        let t_start = t_start.clamp(0.0, 1.0) as f64;
        let t_end = t_end.clamp(0.0, 1.0) as f64;
        self.subsegment(t_start..t_end)
    }
}

impl Trace for BezPath {
    #[inline]
    fn trace_range(&self, t_start: f32, t_end: f32) -> Self {
        trace_bez_path_range(self, t_start, t_end)
    }
}

fn trace_bez_path_range(
    path: &BezPath,
    t_start: f32,
    t_end: f32,
) -> BezPath {
    let t_start = t_start.clamp(0.0, 1.0) as f64;
    let t_end = t_end.clamp(0.0, 1.0) as f64;

    if t_start >= t_end {
        return BezPath::new();
    }
    if t_start <= 0.0 && t_end >= 1.0 {
        return path.clone();
    }

    let n = path.segments().count();
    if n == 0 {
        return BezPath::new();
    }

    let start_scaled = t_start * n as f64;
    let end_scaled = t_end * n as f64;
    let start_seg = start_scaled.floor() as usize;
    let start_frac = start_scaled - start_seg as f64;
    let end_seg = end_scaled.floor() as usize;
    let end_frac = end_scaled - end_seg as f64;

    let mut result = BezPath::new();
    // Track the previous segment's end so we can detect subpath boundaries.
    // `BezPath::segments()` discards `MoveTo` markers, so a new subpath shows
    // up only as a segment whose start doesn't match the previous end. Without
    // re-emitting a `move_to` there, the gap gets drawn as a spurious line.
    let mut last_end: Option<Point> = None;
    for (i, seg) in path.segments().enumerate() {
        if i < start_seg {
            continue;
        }
        if i > end_seg {
            break;
        }
        let lo = if i == start_seg { start_frac } else { 0.0 };
        let hi = if i == end_seg {
            if end_frac == 0.0 {
                break;
            }
            end_frac
        } else {
            1.0
        };
        let sub = seg.subsegment(lo..hi);
        let new_subpath =
            last_end.is_none_or(|end| (sub.start() - end).hypot() > 1e-9);
        if new_subpath {
            result.move_to(sub.start());
        }
        result.push(sub.as_path_el());
        last_end = Some(sub.end());
    }
    result
}
