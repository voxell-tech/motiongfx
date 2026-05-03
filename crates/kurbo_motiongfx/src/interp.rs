use kurbo::{
    Circle, CubicBez, Line, Point, QuadBez, Rect, RoundedRect,
    RoundedRectRadii, Size, Vec2,
};

/// Linearly interpolates between two `f64` values.
#[inline]
pub(crate) fn lerp_f64(a: f64, b: f64, t: f32) -> f64 {
    let t = f64::from(t);
    a * (1.0 - t) + b * t
}

/// Linearly interpolates between two [`Point`]s.
#[inline]
pub fn interp_point(a: &Point, b: &Point, t: f32) -> Point {
    Point::new(lerp_f64(a.x, b.x, t), lerp_f64(a.y, b.y, t))
}

/// Linearly interpolates between two [`Vec2`]s.
#[inline]
pub fn interp_vec2(a: &Vec2, b: &Vec2, t: f32) -> Vec2 {
    Vec2::new(lerp_f64(a.x, b.x, t), lerp_f64(a.y, b.y, t))
}

/// Linearly interpolates between two [`Size`]s.
#[inline]
pub fn interp_size(a: &Size, b: &Size, t: f32) -> Size {
    Size::new(
        lerp_f64(a.width, b.width, t),
        lerp_f64(a.height, b.height, t),
    )
}

/// Linearly interpolates between two [`Rect`]s.
///
/// Each corner coordinate is interpolated independently.
#[inline]
pub fn interp_rect(a: &Rect, b: &Rect, t: f32) -> Rect {
    Rect {
        x0: lerp_f64(a.x0, b.x0, t),
        y0: lerp_f64(a.y0, b.y0, t),
        x1: lerp_f64(a.x1, b.x1, t),
        y1: lerp_f64(a.y1, b.y1, t),
    }
}

/// Linearly interpolates between two [`Circle`]s.
///
/// Both the center position and radius are interpolated.
#[inline]
pub fn interp_circle(a: &Circle, b: &Circle, t: f32) -> Circle {
    Circle {
        center: interp_point(&a.center, &b.center, t),
        radius: lerp_f64(a.radius, b.radius, t),
    }
}

/// Linearly interpolates between two [`Line`]s.
#[inline]
pub fn interp_line(a: &Line, b: &Line, t: f32) -> Line {
    Line {
        p0: interp_point(&a.p0, &b.p0, t),
        p1: interp_point(&a.p1, &b.p1, t),
    }
}

/// Linearly interpolates between two [`CubicBez`] curves.
///
/// Each of the four control points is interpolated independently.
#[inline]
pub fn interp_cubic_bez(
    a: &CubicBez,
    b: &CubicBez,
    t: f32,
) -> CubicBez {
    CubicBez::new(
        interp_point(&a.p0, &b.p0, t),
        interp_point(&a.p1, &b.p1, t),
        interp_point(&a.p2, &b.p2, t),
        interp_point(&a.p3, &b.p3, t),
    )
}

/// Linearly interpolates between two [`QuadBez`] curves.
///
/// Each of the three control points is interpolated independently.
#[inline]
pub fn interp_quad_bez(a: &QuadBez, b: &QuadBez, t: f32) -> QuadBez {
    QuadBez::new(
        interp_point(&a.p0, &b.p0, t),
        interp_point(&a.p1, &b.p1, t),
        interp_point(&a.p2, &b.p2, t),
    )
}

/// Linearly interpolates between two [`RoundedRect`]s.
///
/// Both the bounding rect and each of the four corner radii are
/// interpolated independently.
#[inline]
pub fn interp_rounded_rect(
    a: &RoundedRect,
    b: &RoundedRect,
    t: f32,
) -> RoundedRect {
    let rect = interp_rect(&a.rect(), &b.rect(), t);
    let ra = a.radii();
    let rb = b.radii();
    let radii = RoundedRectRadii::new(
        lerp_f64(ra.top_left, rb.top_left, t),
        lerp_f64(ra.top_right, rb.top_right, t),
        lerp_f64(ra.bottom_right, rb.bottom_right, t),
        lerp_f64(ra.bottom_left, rb.bottom_left, t),
    );
    RoundedRect::from_rect(rect, radii)
}
