use kurbo::{
    Circle, CubicBez, Line, Point, QuadBez, Rect, RoundedRect,
    RoundedRectRadii, Size, Vec2,
};
use motiongfx::prelude::Interpolation;

use crate::Kurbo;

/// Linearly interpolates between two `f64` values.
#[inline]
pub(crate) const fn lerp_f64(a: f64, b: f64, t: f32) -> f64 {
    let t = t as f64;
    a * (1.0 - t) + b * t
}

/// Linearly interpolates between two [`Point`]s.
#[inline]
pub(crate) const fn interp_point(
    a: &Point,
    b: &Point,
    t: f32,
) -> Point {
    Point::new(lerp_f64(a.x, b.x, t), lerp_f64(a.y, b.y, t))
}

impl Interpolation<Kurbo> for Point {
    /// Linearly interpolates between two [`Point`]s.
    #[inline]
    fn interp(a: &Self, b: &Self, t: f32) -> Self {
        interp_point(a, b, t)
    }
}

impl Interpolation<Kurbo> for Vec2 {
    /// Linearly interpolates between two [`Vec2`]s.
    #[inline]
    fn interp(a: &Self, b: &Self, t: f32) -> Self {
        Vec2::new(lerp_f64(a.x, b.x, t), lerp_f64(a.y, b.y, t))
    }
}

impl Interpolation<Kurbo> for Size {
    /// Linearly interpolates between two [`Size`]s.
    #[inline]
    fn interp(a: &Self, b: &Self, t: f32) -> Self {
        Size::new(
            lerp_f64(a.width, b.width, t),
            lerp_f64(a.height, b.height, t),
        )
    }
}

impl Interpolation<Kurbo> for Rect {
    /// Linearly interpolates between two [`Rect`]s.
    ///
    /// Each corner coordinate is interpolated independently.
    #[inline]
    fn interp(a: &Self, b: &Self, t: f32) -> Self {
        Rect {
            x0: lerp_f64(a.x0, b.x0, t),
            y0: lerp_f64(a.y0, b.y0, t),
            x1: lerp_f64(a.x1, b.x1, t),
            y1: lerp_f64(a.y1, b.y1, t),
        }
    }
}

impl Interpolation<Kurbo> for Circle {
    /// Linearly interpolates between two [`Circle`]s.
    ///
    /// Both the center position and radius are interpolated.
    #[inline]
    fn interp(a: &Self, b: &Self, t: f32) -> Self {
        Circle {
            center: interp_point(&a.center, &b.center, t),
            radius: lerp_f64(a.radius, b.radius, t),
        }
    }
}

impl Interpolation<Kurbo> for Line {
    /// Linearly interpolates between two [`Line`]s.
    #[inline]
    fn interp(a: &Self, b: &Self, t: f32) -> Self {
        Line {
            p0: interp_point(&a.p0, &b.p0, t),
            p1: interp_point(&a.p1, &b.p1, t),
        }
    }
}

impl Interpolation<Kurbo> for CubicBez {
    /// Linearly interpolates between two [`CubicBez`] curves.
    ///
    /// Each of the four control points is interpolated independently.
    #[inline]
    fn interp(a: &Self, b: &Self, t: f32) -> Self {
        CubicBez::new(
            interp_point(&a.p0, &b.p0, t),
            interp_point(&a.p1, &b.p1, t),
            interp_point(&a.p2, &b.p2, t),
            interp_point(&a.p3, &b.p3, t),
        )
    }
}

impl Interpolation<Kurbo> for QuadBez {
    /// Linearly interpolates between two [`QuadBez`] curves.
    ///
    /// Each of the three control points is interpolated independently.
    #[inline]
    fn interp(a: &Self, b: &Self, t: f32) -> Self {
        QuadBez::new(
            interp_point(&a.p0, &b.p0, t),
            interp_point(&a.p1, &b.p1, t),
            interp_point(&a.p2, &b.p2, t),
        )
    }
}

impl Interpolation<Kurbo> for RoundedRect {
    /// Linearly interpolates between two [`RoundedRect`]s.
    ///
    /// Both the bounding rect and each of the four corner radii are
    /// interpolated independently.
    #[inline]
    fn interp(a: &Self, b: &Self, t: f32) -> Self {
        let rect = Rect::interp(&a.rect(), &b.rect(), t);
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
}
