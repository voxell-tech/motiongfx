use bevy_vello_renderer::vello::peniko;

use crate::lerp::Lerp;

pub trait CrossLerp<Time, Other, Return> {
    fn cross_lerp(&self, other: &Other, t: Time) -> Return;
}

impl CrossLerp<f32, peniko::ColorStops, peniko::ColorStops> for peniko::Color {
    fn cross_lerp(&self, other: &peniko::ColorStops, t: f32) -> peniko::ColorStops {
        let self_stops = peniko::ColorStops::from_vec(vec![peniko::ColorStop {
            offset: 0.0,
            color: *self,
        }]);

        peniko::ColorStops::lerp(&self_stops, other, t)
    }
}

impl CrossLerp<f32, peniko::Color, peniko::ColorStops> for peniko::ColorStops {
    fn cross_lerp(&self, other: &peniko::Color, t: f32) -> peniko::ColorStops {
        let other_stops = peniko::ColorStops::from_vec(vec![peniko::ColorStop {
            offset: 0.0,
            color: *other,
        }]);

        peniko::ColorStops::lerp(self, &other_stops, t)
    }
}
