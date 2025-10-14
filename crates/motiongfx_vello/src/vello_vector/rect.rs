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

#[derive(Component, Clone, Default)]
pub struct VelloRect {
    /// Coordinates of the rectangle.
    pub(crate) rect: kurbo::Rect,
    /// Radius of all four corners.
    pub(crate) radii: kurbo::RoundedRectRadii,
    built: bool,
}

impl VelloRect {
    #[inline]
    pub fn new(rect: impl Into<kurbo::Rect>, radii: impl Into<kurbo::RoundedRectRadii>) -> Self {
        let rect: kurbo::Rect = rect.into();
        let radii: kurbo::RoundedRectRadii = radii.into();

        Self {
            rect,
            radii,
            ..default()
        }
    }

    pub fn from_vec(points: impl Into<DVec4>, radii: impl Into<DVec4>) -> Self {
        let points: DVec4 = points.into();
        let radii: DVec4 = radii.into();

        Self::new(
            kurbo::Rect::new(points.x, points.y, points.z, points.w),
            kurbo::RoundedRectRadii::new(radii.x, radii.y, radii.z, radii.w),
        )
    }

    pub fn with_rect(mut self, rect: impl Into<kurbo::Rect>) -> Self {
        let rect: kurbo::Rect = rect.into();

        self.rect = rect;
        self
    }

    pub fn with_radii(mut self, radii: impl Into<kurbo::RoundedRectRadii>) -> Self {
        let radii: kurbo::RoundedRectRadii = radii.into();

        self.radii = radii;
        self
    }

    pub fn percentage_anchor(
        size: impl Into<DVec2>,
        radius: impl Into<DVec4>,
        percentage: impl Into<DVec2>,
    ) -> Self {
        let size: DVec2 = size.into();
        let radius: DVec4 = radius.into();
        let percentage: DVec2 = percentage.into();

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
    pub fn anchor_left(size: impl Into<DVec2>, radius: impl Into<DVec4>) -> Self {
        Self::percentage_anchor(size, radius, DVec2::new(1.0, 0.5))
    }

    #[inline]
    pub fn anchor_right(size: impl Into<DVec2>, radius: impl Into<DVec4>) -> Self {
        Self::percentage_anchor(size, radius, DVec2::new(0.0, 0.5))
    }

    #[inline]
    pub fn anchor_bottom(size: impl Into<DVec2>, radius: impl Into<DVec4>) -> Self {
        Self::percentage_anchor(size, radius, DVec2::new(0.5, 0.0))
    }

    #[inline]
    pub fn anchor_top(size: impl Into<DVec2>, radius: impl Into<DVec4>) -> Self {
        Self::percentage_anchor(size, radius, DVec2::new(0.5, 1.0))
    }

    #[inline]
    pub fn anchor_center(size: impl Into<DVec2>, radius: impl Into<DVec4>) -> Self {
        Self::percentage_anchor(size, radius, DVec2::new(0.5, 0.5))
    }
}

impl VelloVector for VelloRect {
    #[inline]
    fn shape(&self) -> &impl kurbo::Shape {
        &self.rect
    }
}

impl VelloBuilder for VelloRect {
    #[inline]
    fn is_built(&self) -> bool {
        self.built
    }

    #[inline]
    fn set_built(&mut self, built: bool) {
        self.built = built;
    }
}
