use bevy::prelude::*;
use bevy_vello_graphics::bevy_vello::vello::{kurbo, peniko};

use super::F32Lerp;

impl F32Lerp for kurbo::Stroke {
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        Self {
            width: f64::lerp(self.width, rhs.width, t as f64),
            miter_limit: f64::lerp(self.miter_limit, rhs.miter_limit, t as f64),
            dash_offset: f64::lerp(self.dash_offset, rhs.dash_offset, t as f64),
            // dash_pattern: kurbo::Dashes::lerp(&self.dash_pattern, &other.dash_pattern, t),
            ..default()
        }
    }
}

impl F32Lerp<peniko::ColorStops, peniko::ColorStops> for peniko::Color {
    fn f32lerp(&self, rhs: &peniko::ColorStops, t: f32) -> peniko::ColorStops {
        let self_stops = peniko::ColorStops::from(
            [peniko::ColorStop {
                offset: 0.0,
                color: peniko::color::DynamicColor::from_alpha_color(*self),
            }]
            .as_slice(),
        );

        bevy_vello_graphics::bevy_vello::prelude::peniko::ColorStops(smallvec::SmallVec::f32lerp(
            &self_stops.0,
            &rhs.0,
            t,
        ))
    }
}

impl F32Lerp<peniko::Color, peniko::ColorStops> for peniko::ColorStops {
    fn f32lerp(&self, rhs: &peniko::Color, t: f32) -> peniko::ColorStops {
        let other_stops = peniko::ColorStops::from(
            [peniko::ColorStop {
                offset: 0.0,
                color: peniko::color::DynamicColor::from_alpha_color(*rhs),
            }]
            .as_slice(),
        );

        bevy_vello_graphics::bevy_vello::prelude::peniko::ColorStops(smallvec::SmallVec::f32lerp(
            self,
            &other_stops,
            t,
        ))
    }
}

impl F32Lerp for peniko::Brush {
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        match self {
            Self::Solid(self_color) => match rhs {
                // Solid -> Solid
                Self::Solid(other_color) => {
                    return Self::Solid(peniko::Color::f32lerp(self_color, other_color, t));
                }

                // Solid -> Gradient
                Self::Gradient(other_grad) => {
                    return Self::Gradient(peniko::Gradient {
                        kind: other_grad.kind,
                        extend: other_grad.extend,
                        stops: peniko::Color::f32lerp(self_color, &other_grad.stops, t),
                        ..Default::default()
                    });
                }

                Self::Image(_) => {
                    panic!("Image interpolation is not supported.");
                }
            },

            Self::Gradient(self_grad) => match rhs {
                // Gradient -> Solid
                Self::Solid(other_color) => {
                    return Self::Gradient(peniko::Gradient {
                        kind: self_grad.kind,
                        extend: self_grad.extend,
                        stops: peniko::ColorStops::f32lerp(&self_grad.stops, other_color, t),
                        ..Default::default()
                    });
                }

                // Gradient -> Gradient
                Self::Gradient(other_grad) => 'grad: {
                    // Gradient kind and extend must be the same, otherwise, fallback
                    if self_grad.kind != other_grad.kind && self_grad.extend != other_grad.extend {
                        break 'grad;
                    }

                    return Self::Gradient(peniko::Gradient {
                        kind: self_grad.kind,
                        extend: self_grad.extend,
                        stops: peniko::ColorStops::f32lerp(&self_grad.stops, &other_grad.stops, t),
                        ..Default::default()
                    });
                }

                Self::Image(_) => {
                    panic!("Image interpolation is not supported.");
                }
            },

            Self::Image(_) => {
                panic!("Image interpolation is not supported.");
            }
        }

        // Fallback to discrete interpolation
        if t < 0.5 {
            self.clone()
        } else {
            rhs.clone()
        }
    }
}

impl<Item, Array> F32Lerp for smallvec::SmallVec<Array>
where
    Item: F32Lerp + Default + PartialEq + Clone,
    Array: smallvec::Array<Item = Item>,
{
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        let mut self_iter = self.iter();
        let mut other_iter = rhs.iter();

        let mut last_self_item = Item::default();
        let mut last_other_item = Item::default();

        let mut interp_vec = smallvec::SmallVec::new();

        loop {
            let self_item = self_iter.next();
            let other_item = other_iter.next();

            if self_item.is_none() && other_item.is_none() {
                break;
            }

            last_self_item = self_item.unwrap_or(&last_self_item).clone();
            last_other_item = other_item.unwrap_or(&last_other_item).clone();

            interp_vec.push(Item::f32lerp(&last_self_item, &last_other_item, t));
        }

        interp_vec
    }
}

impl F32Lerp for peniko::ColorStop {
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        Self {
            offset: f32::lerp(self.offset, rhs.offset, t),
            color: peniko::color::DynamicColor::from_alpha_color(peniko::Color::f32lerp(
                &self.color.to_alpha_color(),
                &rhs.color.to_alpha_color(),
                t,
            )),
        }
    }
}

impl F32Lerp for peniko::Color {
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        self.lerp_rect(*rhs, t)
    }
}
