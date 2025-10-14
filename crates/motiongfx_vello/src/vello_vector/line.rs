use bevy_ecs::prelude::*;
use bevy_math::DVec2;
use bevy_utils::prelude::*;
use bevy_vello_renderer::{prelude::*, vello::kurbo};

use crate::{
    stroke_style::StrokeStyle,
    vello_vector::{VelloBuilder, VelloVector},
};

#[derive(Bundle, Clone, Default)]
pub struct VelloLineBundle {
    pub line: VelloLine,
    pub stroke: StrokeStyle,
    pub scene_bundle: VelloSceneBundle,
}

#[derive(VelloBuilder, VelloVector, Component, Clone)]
pub struct VelloLine {
    #[shape]
    pub line: kurbo::Line,
    built: bool,
}

impl VelloLine {
    #[inline]
    pub fn new(line: kurbo::Line) -> Self {
        Self { line, ..default() }
    }

    pub fn from_points(p0: DVec2, p1: DVec2) -> Self {
        Self {
            line: kurbo::Line::new(kurbo::Point::new(p0.x, p0.y), kurbo::Point::new(p1.x, p1.y)),
            ..default()
        }
    }

    pub fn origin_to(to: DVec2) -> Self {
        let to: DVec2 = to.into();

        Self {
            line: kurbo::Line::new(kurbo::Point::default(), kurbo::Point::new(to.x, to.y)),
            ..default()
        }
    }
}

impl Default for VelloLine {
    fn default() -> Self {
        Self {
            line: kurbo::Line::new(kurbo::Point::default(), kurbo::Point::default()),
            built: false,
        }
    }
}
