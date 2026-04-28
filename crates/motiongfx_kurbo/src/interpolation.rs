use kurbo::{Affine, Circle, Line, Point, Rect, RoundedRect, RoundedRectRadii, Size, Vec2};

#[inline]
fn lerp(a: f64, b: f64, t: f32) -> f64 {
    let t = f64::from(t);
    a * (1.0 - t) + b * t
}

#[inline]
pub fn interp_point(a: &Point, b: &Point, t: f32) -> Point {
    Point::new(lerp(a.x, b.x, t), lerp(a.y, b.y, t))
}

#[inline]
pub fn interp_vec2(a: &Vec2, b: &Vec2, t: f32) -> Vec2 {
    Vec2::new(lerp(a.x, b.x, t), lerp(a.y, b.y, t))
}

#[inline]
pub fn interp_size(a: &Size, b: &Size, t: f32) -> Size {
    Size::new(lerp(a.width, b.width, t), lerp(a.height, b.height, t))
}

#[inline]
pub fn interp_rect(a: &Rect, b: &Rect, t: f32) -> Rect {
    Rect {
        x0: lerp(a.x0, b.x0, t),
        y0: lerp(a.y0, b.y0, t),
        x1: lerp(a.x1, b.x1, t),
        y1: lerp(a.y1, b.y1, t),
    }
}

#[inline]
pub fn interp_circle(a: &Circle, b: &Circle, t: f32) -> Circle {
    Circle {
        center: interp_point(&a.center, &b.center, t),
        radius: lerp(a.radius, b.radius, t),
    }
}

#[inline]
pub fn interp_line(a: &Line, b: &Line, t: f32) -> Line {
    Line {
        p0: interp_point(&a.p0, &b.p0, t),
        p1: interp_point(&a.p1, &b.p1, t),
    }
}

/// Linearly interpolates between two [`Affine`] transforms component-wise.
#[inline]
pub fn interp_affine(a: &Affine, b: &Affine, t: f32) -> Affine {
    let ac = a.as_coeffs();
    let bc = b.as_coeffs();
    Affine::new([
        lerp(ac[0], bc[0], t),
        lerp(ac[1], bc[1], t),
        lerp(ac[2], bc[2], t),
        lerp(ac[3], bc[3], t),
        lerp(ac[4], bc[4], t),
        lerp(ac[5], bc[5], t),
    ])
}

#[inline]
pub fn interp_rounded_rect(a: &RoundedRect, b: &RoundedRect, t: f32) -> RoundedRect {
    let rect = interp_rect(&a.rect(), &b.rect(), t);
    let ra = a.radii();
    let rb = b.radii();
    let radii = RoundedRectRadii::new(
        lerp(ra.top_left, rb.top_left, t),
        lerp(ra.top_right, rb.top_right, t),
        lerp(ra.bottom_right, rb.bottom_right, t),
        lerp(ra.bottom_left, rb.bottom_left, t),
    );
    RoundedRect::from_rect(rect, radii)
}
