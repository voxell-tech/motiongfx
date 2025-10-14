use bevy_ecs::prelude::*;
use bevy_math::{DVec2, DVec4};
use bevy_utils::prelude::*;
use bevy_vello_renderer::{prelude::*, vello::kurbo};

use crate::{
    fill_style::FillStyle,
    stroke_style::StrokeStyle,
    vello_vector::{VelloBuilder, VelloVector},
};

#[derive(Bundle, Clone, Default)]
pub struct VelloRectBundle {
    pub rect: VelloRect,
    pub fill: FillStyle,
    pub stroke: StrokeStyle,
    pub scene_bundle: VelloSceneBundle,
}

#[derive(VelloBuilder, Component, Clone, Default)]
pub struct VelloRect {
    /// Coordinates of the rectangle.
    pub rect: kurbo::Rect,
    /// Radius of all four corners.
    pub radii: kurbo::RoundedRectRadii,
    built: bool,
}

impl VelloRect {
    #[inline]
    pub fn new(rect: kurbo::Rect, radii: impl Into<kurbo::RoundedRectRadii>) -> Self {
        let radii: kurbo::RoundedRectRadii = radii.into();

        Self {
            rect,
            radii,
            ..default()
        }
    }

    pub fn from_vec(points: DVec4, radii: DVec4) -> Self {
        Self::new(
            kurbo::Rect::new(points.x, points.y, points.z, points.w),
            kurbo::RoundedRectRadii::new(radii.x, radii.y, radii.z, radii.w),
        )
    }

    pub fn with_rect(mut self, rect: kurbo::Rect) -> Self {
        self.rect = rect;
        self
    }

    pub fn with_radii(mut self, radii: impl Into<kurbo::RoundedRectRadii>) -> Self {
        self.radii = radii.into();
        self
    }

    pub fn percentage_anchor(size: DVec2, radius: DVec4, percentage: DVec2) -> Self {
        Self::new(
            kurbo::Rect::new(
                -size.x * percentage.x,
                -size.y * percentage.y,
                size.x * (1.0 - percentage.x),
                size.y * (1.0 - percentage.y),
            ),
            kurbo::RoundedRectRadii::new(radius.x, radius.y, radius.z, radius.w),
        )
    }

    #[inline]
    pub fn anchor_left(size: DVec2, radius: DVec4) -> Self {
        Self::percentage_anchor(size, radius, DVec2::new(1.0, 0.5))
    }

    #[inline]
    pub fn anchor_right(size: DVec2, radius: DVec4) -> Self {
        Self::percentage_anchor(size, radius, DVec2::new(0.0, 0.5))
    }

    #[inline]
    pub fn anchor_bottom(size: DVec2, radius: DVec4) -> Self {
        Self::percentage_anchor(size, radius, DVec2::new(0.5, 0.0))
    }

    #[inline]
    pub fn anchor_top(size: DVec2, radius: DVec4) -> Self {
        Self::percentage_anchor(size, radius, DVec2::new(0.5, 1.0))
    }

    #[inline]
    pub fn anchor_center(size: DVec2, radius: DVec4) -> Self {
        Self::percentage_anchor(size, radius, DVec2::new(0.5, 0.5))
    }
}

impl VelloVector for VelloRect {
    #[inline]
    fn shape(&self) -> impl kurbo::Shape {
        kurbo::RoundedRect::from_rect(self.rect, self.radii)
    }
}
