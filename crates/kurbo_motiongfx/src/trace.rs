use kurbo::{
    BezPath, CubicBez, ParamCurve, PathEl, PathSeg, QuadBez,
};

#[inline]
pub fn trace_cubic_bez(curve: &CubicBez, t: f32) -> CubicBez {
    let t = t as f64;
    curve.subsegment(0.0..t)
}

#[inline]
pub fn trace_quad_bez(curve: &QuadBez, t: f32) -> QuadBez {
    let t = t as f64;
    curve.subsegment(0.0..t)
}

/// Returns the prefix of `path` traced from the start up to progress `t ∈ [0, 1]`.
///
/// Progress is distributed uniformly across segments (not arc-length
/// parameterised). At `t = 0.0` the result is empty; at `t = 1.0` it
/// is a full clone of `path`.
///
/// Only single-subpath paths are handled correctly. Multi-subpath paths
/// will be traced as if they were a single continuous stroke.
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
    for (i, seg) in path.segments().enumerate() {
        if i == 0 {
            result.move_to(seg.start());
        }
        if i < complete {
            push_seg(&mut result, seg);
        } else if i == complete && partial_t > 0.0 {
            push_seg(&mut result, seg.subsegment(0.0..partial_t));
            break;
        } else {
            break;
        }
    }
    result
}

#[inline]
fn push_seg(path: &mut BezPath, seg: PathSeg) {
    match seg {
        PathSeg::Line(l) => path.push(PathEl::LineTo(l.p1)),
        PathSeg::Quad(q) => path.push(PathEl::QuadTo(q.p1, q.p2)),
        PathSeg::Cubic(c) => {
            path.push(PathEl::CurveTo(c.p1, c.p2, c.p3))
        }
    }
}
