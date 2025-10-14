use bevy_ecs::prelude::*;
use bevy_math::DVec2;
use bevy_utils::prelude::*;
use bevy_vello_renderer::{prelude::*, vello::kurbo};

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
    pub scene_bundle: VelloSceneBundle,
}

#[derive(VelloBuilder, VelloVector, Component, Clone, Default)]
pub struct VelloCircle {
    #[shape]
    pub circle: kurbo::Circle,
    built: bool,
}

impl VelloCircle {
    #[inline]
    pub fn new(circle: kurbo::Circle) -> Self {
        Self {
            circle,
            ..default()
        }
    }

    pub fn from_vec(center: DVec2, radius: f64) -> Self {
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
