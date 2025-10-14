use bevy_render::prelude::*;
use bevy_utils::prelude::*;
use bevy_vello_renderer::vello::{kurbo, peniko};

use crate::CrossLerp;

pub trait Lerp<Time> {
    fn lerp(&self, other: &Self, t: Time) -> Self;
}

impl Lerp<f32> for kurbo::RoundedRectRadii {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            top_left: f64::lerp(&self.top_left, &other.top_left, t),
            top_right: f64::lerp(&self.top_right, &other.top_right, t),
            bottom_right: f64::lerp(&self.bottom_right, &other.bottom_right, t),
            bottom_left: f64::lerp(&self.bottom_left, &other.bottom_left, t),
        }
    }
}

impl Lerp<f32> for kurbo::Rect {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            x0: f64::lerp(&self.x0, &other.x0, t),
            y0: f64::lerp(&self.y0, &other.y0, t),
            x1: f64::lerp(&self.x1, &other.x1, t),
            y1: f64::lerp(&self.y1, &other.y1, t),
        }
    }
}

impl Lerp<f32> for kurbo::Circle {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            center: kurbo::Point::lerp(self.center, other.center, t as f64),
            radius: f64::lerp(&self.radius, &other.radius, t),
        }
    }
}

impl Lerp<f32> for kurbo::Line {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let t: f64 = t as f64;

        Self {
            p0: kurbo::Point::lerp(self.p0, other.p0, t),
            p1: kurbo::Point::lerp(self.p1, other.p1, t),
        }
    }
}

impl Lerp<f32> for kurbo::Stroke {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            width: f64::lerp(&self.width, &other.width, t),
            miter_limit: f64::lerp(&self.miter_limit, &other.miter_limit, t),
            dash_offset: f64::lerp(&self.dash_offset, &other.dash_offset, t),
            dash_pattern: kurbo::Dashes::lerp(&self.dash_pattern, &other.dash_pattern, t),
            ..default()
        }
    }
}

impl Lerp<f32> for peniko::Brush {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        match self {
            Self::Solid(self_color) => match other {
                // =====================
                // Solid -> Solid
                // =====================
                Self::Solid(other_color) => {
                    return Self::Solid(peniko::Color::lerp(self_color, other_color, t));
                }

                // =====================
                // Solid -> Gradient
                // =====================
                Self::Gradient(other_grad) => {
                    return Self::Gradient(peniko::Gradient {
                        kind: other_grad.kind,
                        extend: other_grad.extend,
                        stops: peniko::Color::cross_lerp(self_color, &other_grad.stops, t),
                    });
                }

                // Image interpolation is not supported
                Self::Image(_) => {}
            },

            Self::Gradient(self_grad) => match other {
                // =====================
                // Gradient -> Solid
                // =====================
                Self::Solid(other_color) => {
                    return Self::Gradient(peniko::Gradient {
                        kind: self_grad.kind,
                        extend: self_grad.extend,
                        stops: peniko::ColorStops::cross_lerp(&self_grad.stops, other_color, t),
                    });
                }

                // =====================
                // Gradient -> Gradient
                // =====================
                Self::Gradient(other_grad) => 'grad: {
                    // Gradient kind and extend must be the same, otherwise, fallback
                    if self_grad.kind != other_grad.kind && self_grad.extend != other_grad.extend {
                        break 'grad;
                    }

                    return Self::Gradient(peniko::Gradient {
                        kind: self_grad.kind,
                        extend: self_grad.extend,
                        stops: peniko::ColorStops::lerp(&self_grad.stops, &other_grad.stops, t),
                    });
                }

                // Image interpolation is not supported
                Self::Image(_) => {}
            },

            // Image interpolation is not supported
            Self::Image(_) => {}
        }

        // Fallback to discrete interpolation
        if t < 0.5 {
            self.clone()
        } else {
            other.clone()
        }
    }
}

impl<Item, Array> Lerp<f32> for smallvec::SmallVec<Array>
where
    Item: Lerp<f32> + Default + PartialEq + Clone,
    Array: smallvec::Array<Item = Item>,
{
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let mut self_iter: std::slice::Iter<Item> = self.iter();
        let mut other_iter: std::slice::Iter<Item> = other.iter();

        let mut last_self_item: Item = Item::default();
        let mut last_other_item: Item = Item::default();

        let mut interp_vec: smallvec::SmallVec<Array> = smallvec::SmallVec::new();

        loop {
            let self_item: Option<&Item> = self_iter.next();
            let other_item: Option<&Item> = other_iter.next();

            if self_item.is_none() && other_item.is_none() {
                break;
            }

            last_self_item = self_item.unwrap_or(&last_self_item).clone();
            last_other_item = other_item.unwrap_or(&last_other_item).clone();

            interp_vec.push(Item::lerp(&last_self_item, &last_other_item, t));
        }

        interp_vec
    }
}

impl Lerp<f32> for peniko::ColorStop {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            offset: f32::lerp(&self.offset, &other.offset, t),
            color: peniko::Color::lerp(&self.color, &other.color, t),
        }
    }
}

impl Lerp<f32> for peniko::Color {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::rgba8(
            u8::lerp(&self.r, &other.r, t),
            u8::lerp(&self.g, &other.g, t),
            u8::lerp(&self.b, &other.b, t),
            u8::lerp(&self.a, &other.a, t),
        )
    }
}

impl Lerp<f32> for Color {
    #[inline]
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::rgba(
            f32::lerp(&self.r(), &other.r(), t),
            f32::lerp(&self.g(), &other.g(), t),
            f32::lerp(&self.b(), &other.b(), t),
            f32::lerp(&self.a(), &other.a(), t),
        )
    }
}

impl Lerp<f32> for f32 {
    #[inline]
    fn lerp(&self, other: &Self, t: f32) -> Self {
        (other - self) * t + self
    }
}

impl Lerp<f64> for f64 {
    #[inline]
    fn lerp(&self, other: &Self, t: f64) -> Self {
        (other - self) * t + self
    }
}

impl Lerp<f32> for f64 {
    #[inline]
    fn lerp(&self, other: &Self, t: f32) -> Self {
        (other - self) * (t as f64) + self
    }
}

impl Lerp<f32> for u8 {
    #[inline]
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let other: f32 = *other as f32;
        let self_: f32 = *self as f32;

        ((other - self_) * t + self_) as u8
    }
}
