use bevy_ecs::prelude::*;
use bevy_math::DVec2;
use bevy_utils::prelude::*;
use bevy_vello_renderer::{
    prelude::*,
    vello::{self, kurbo},
};

use crate::{
    fill_style::FillStyle,
    stroke_style::StrokeStyle,
    vello_vector::{VelloBuilder, VelloVector},
};

#[derive(Bundle, Clone, Default)]
pub struct VelloCircleBundle {
    pub circle: VelloCircle,
    pub fill: FillStyle,
    pub stroke: StrokeStyle,
    pub fragment_bundle: VelloFragmentBundle,
}

#[derive(Component, Clone, Default)]
pub struct VelloCircle {
    pub(crate) circle: kurbo::Circle,
    built: bool,
}

impl VelloCircle {
    #[inline]
    pub fn new(circle: impl Into<kurbo::Circle>) -> Self {
        let circle: kurbo::Circle = circle.into();

        Self {
            circle,
            ..default()
        }
    }

    pub fn from_vec(center: impl Into<DVec2>, radius: f64) -> Self {
        let center: DVec2 = center.into();

        Self::new(kurbo::Circle::new(
            kurbo::Point::new(center.x, center.y),
            radius,
        ))
    }

    #[inline]
    pub fn from_radius(radius: f64) -> Self {
        Self::new(kurbo::Circle::new(kurbo::Point::default(), radius))
    }
}

impl VelloVector for VelloCircle {
    fn build_fill(&self, fill: &FillStyle, builder: &mut vello::SceneBuilder) {
        builder.fill(
            fill.style,
            kurbo::Affine::default(),
            &fill.brush,
            None,
            &self.circle,
        );
    }

    fn build_stroke(&self, stroke: &StrokeStyle, builder: &mut vello::SceneBuilder) {
        builder.stroke(
            &stroke.style,
            kurbo::Affine::default(),
            &stroke.brush,
            None,
            &self.circle,
        );
    }
}

impl VelloBuilder for VelloCircle {
    #[inline]
    fn is_built(&self) -> bool {
        self.built
    }

    #[inline]
    fn set_built(&mut self, built: bool) {
        self.built = built
    }
}
