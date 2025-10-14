use bevy_asset::Assets;
use bevy_ecs::prelude::*;
use bevy_math::{DVec2, DVec4};
use bevy_utils::prelude::*;
use bevy_vello_renderer::{prelude::*, vello::kurbo};

use crate::{
    convert::PenikoBrush,
    fill_style::FillStyle,
    stroke_style::StrokeStyle,
    vello_motion::rect_motion::VelloRectBundleMotion,
    vello_vector::{VelloBuilder, VelloVector},
};

#[derive(Bundle, Clone, Default)]
pub struct VelloRectBundle {
    pub rect: VelloRect,
    pub fill: FillStyle,
    pub stroke: StrokeStyle,
    pub scene_bundle: VelloSceneBundle,
}

#[derive(Default)]
pub enum RectAnchor {
    #[default]
    Center,
    Left,
    Right,
    Bottom,
    Top,
}

#[derive(Default)]
pub struct VelloRectBuilder {
    pub size: DVec2,
    pub radii: DVec4,
    pub anchor: RectAnchor,
    pub fill_brush: PenikoBrush,
    pub stroke_brush: PenikoBrush,
}

impl VelloRectBuilder {
    pub fn size(mut self, width: f64, height: f64) -> Self {
        self.size.x = width;
        self.size.y = height;

        self
    }

    pub fn radius(mut self, radius: f64) -> Self {
        self.radii = DVec4::splat(radius);

        self
    }

    pub fn radii(mut self, radius: DVec4) -> Self {
        self.radii = radius;

        self
    }

    pub fn anchor(mut self, anchor: RectAnchor) -> Self {
        self.anchor = anchor;

        self
    }

    pub fn fill(mut self, fill_brush: impl Into<PenikoBrush>) -> Self {
        self.fill_brush = fill_brush.into();

        self
    }

    pub fn stroke(mut self, stroke_brush: impl Into<PenikoBrush>) -> Self {
        self.stroke_brush = stroke_brush.into();

        self
    }

    pub fn build(
        self,
        commands: &mut Commands,
        scenes: &mut Assets<VelloScene>,
    ) -> VelloRectBundleMotion {
        let rect = match self.anchor {
            RectAnchor::Center => VelloRect::anchor_center(self.size, self.radii),
            RectAnchor::Left => VelloRect::anchor_left(self.size, self.radii),
            RectAnchor::Right => VelloRect::anchor_right(self.size, self.radii),
            RectAnchor::Bottom => VelloRect::anchor_bottom(self.size, self.radii),
            RectAnchor::Top => VelloRect::anchor_top(self.size, self.radii),
        };

        let rect_bundle = VelloRectBundle {
            rect,
            fill: FillStyle::from_brush(self.fill_brush),
            stroke: StrokeStyle::from_brush(self.stroke_brush),
            scene_bundle: VelloSceneBundle {
                scene: scenes.add(VelloScene::default()),
                ..default()
            },
        };

        let rect_id = commands.spawn(rect_bundle.clone()).id();

        VelloRectBundleMotion::new(rect_id, rect_bundle)
    }
}

pub fn create_rect(width: f64, height: f64) -> VelloRectBuilder {
    VelloRectBuilder::default().size(width, height)
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
    pub fn anchor_center(size: DVec2, radius: DVec4) -> Self {
        Self::percentage_anchor(size, radius, DVec2::new(0.5, 0.5))
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
}

impl VelloVector for VelloRect {
    #[inline]
    fn shape(&self) -> impl kurbo::Shape {
        kurbo::RoundedRect::from_rect(self.rect, self.radii)
    }
}
