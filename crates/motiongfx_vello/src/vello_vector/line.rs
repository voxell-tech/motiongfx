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

#[derive(Component, Clone)]
pub struct VelloLine {
    pub(crate) line: kurbo::Line,
    built: bool,
}

impl VelloLine {
    #[inline]
    pub fn new(line: impl Into<kurbo::Line>) -> Self {
        let line: kurbo::Line = line.into();

        Self { line, ..default() }
    }

    pub fn from_points(p0: impl Into<DVec2>, p1: impl Into<DVec2>) -> Self {
        let p0: DVec2 = p0.into();
        let p1: DVec2 = p1.into();

        Self {
            line: kurbo::Line::new(kurbo::Point::new(p0.x, p0.y), kurbo::Point::new(p1.x, p1.y)),
            ..default()
        }
    }

    pub fn origin_to(to: impl Into<DVec2>) -> Self {
        let to: DVec2 = to.into();

        Self {
            line: kurbo::Line::new(kurbo::Point::default(), kurbo::Point::new(to.x, to.y)),
            ..default()
        }
    }
}

impl VelloVector for VelloLine {
    #[inline]
    fn shape(&self) -> &impl kurbo::Shape {
        &self.line
    }
}

impl VelloBuilder for VelloLine {
    #[inline]
    fn is_built(&self) -> bool {
        self.built
    }

    #[inline]
    fn set_built(&mut self, built: bool) {
        self.built = built;
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
