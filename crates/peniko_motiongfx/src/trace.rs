use peniko::kurbo::{BezPath, CubicBez, Line, ParamCurve, QuadBez};

pub type LineTracer = Tracer<Line>;
pub type QuadTracer = Tracer<QuadBez>;
pub type CubicTracer = Tracer<CubicBez>;
pub type PathTracer = Tracer<BezPath>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tracer<T: Trace> {
    /// The original full path.
    pub path: T,
    /// The `t` value to trace `path`.
    pub t: f32,
}

impl<T: Trace> Tracer<T> {
    pub fn trace(&self) -> T {
        self.path.trace(self.t)
    }
}

pub trait Trace {
    fn trace(&self, t: f32) -> Self;
}

impl Trace for Line {
    /// Returns the prefix of a [`Line`] traced from start to `t`.
    #[inline]
    fn trace(&self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0) as f64;
        self.subsegment(0.0..t)
    }
}

impl Trace for QuadBez {
    /// Returns the prefix of a [`QuadBez`] traced from start to `t`.
    #[inline]
    fn trace(&self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0) as f64;
        self.subsegment(0.0..t)
    }
}

impl Trace for CubicBez {
    /// Returns the prefix of a [`CubicBez`] traced from start to `t`.
    #[inline]
    fn trace(&self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0) as f64;
        self.subsegment(0.0..t)
    }
}

impl Trace for BezPath {
    /// See [`trace_bez_path`].
    #[inline]
    fn trace(&self, t: f32) -> Self {
        trace_bez_path(self, t)
    }
}

/// Returns the prefix of `path` traced from the start to `t`.
///
/// Progress is distributed uniformly across segments (not
/// arc-length parameterised).
///
/// Trace multiple paths as if they were a single continuous stroke.
pub fn trace_bez_path(path: &BezPath, t: f32) -> BezPath {
    if t <= 0.0 {
        return BezPath::new();
    }
    if t >= 1.0 {
        return path.clone();
    }

    let n = path.segments().count();
    if n == 0 {
        return BezPath::new();
    }

    let t = f64::from(t);
    let scaled = t * n as f64;
    let complete = scaled.floor() as usize;
    let partial_t = scaled - complete as f64;

    let mut result = BezPath::new();
    result.move_to(
        path.get_seg(0).map(|s| s.start()).unwrap_or_default(),
    );
    for seg in path.segments().take(complete) {
        result.push(seg.as_path_el());
    }
    if partial_t > 0.0
        && let Some(seg) = path.segments().nth(complete)
    {
        result.push(seg.subsegment(0.0..partial_t).as_path_el());
    }

    result
}
