use bevy_ecs::prelude::*;
use bevy_math::{DVec2, DVec4};
use bevy_utils::prelude::*;
use bevy_vello_renderer::{
    prelude::*,
    vello::{self, kurbo, peniko},
};
use motiongfx_core::prelude::*;

use crate::{
    convert::*,
    vector_style::{FillStyle, StrokeStyle},
    vello_vector::VelloVector,
};

#[derive(Bundle, Clone, Default)]
pub struct VelloRectBundle {
    pub rect: VelloRect,
    pub fragment_bundle: VelloFragmentBundle,
}

#[derive(Component, Clone, Default)]
pub struct VelloRect {
    /// Coordinates of the rectangle.
    rect: kurbo::Rect,
    /// Radius of all four corners.
    radii: kurbo::RoundedRectRadii,
    fill: FillStyle,
    stroke: StrokeStyle,
    should_build: bool,
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

    pub fn with_fill(mut self, fill: FillStyle) -> Self {
        self.fill = fill;
        self
    }

    pub fn with_fill_style(mut self, style: peniko::Fill) -> Self {
        self.fill.style = style;
        self
    }

    pub fn with_fill_brush(mut self, brush: impl Into<PenikoBrush>) -> Self {
        self.fill.brush = brush.into().0;
        self
    }

    pub fn with_stroke(mut self, stroke: StrokeStyle) -> Self {
        self.stroke = stroke;
        self
    }

    pub fn with_stroke_style(mut self, style: impl Into<KurboStroke>) -> VelloRect {
        self.stroke.style = style.into().0;
        self
    }

    pub fn with_stroke_brush(mut self, brush: impl Into<PenikoBrush>) -> Self {
        self.stroke.brush = brush.into().0;
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
    fn build(&self, fragment: &mut VelloFragment) {
        let mut frag: vello::SceneFragment = vello::SceneFragment::new();
        let mut sb: vello::SceneBuilder = vello::SceneBuilder::for_fragment(&mut frag);

        sb.fill(
            self.fill.style,
            kurbo::Affine::default(),
            &self.fill.brush,
            None,
            &kurbo::RoundedRect::from_rect(self.rect, self.radii),
        );

        sb.stroke(
            &self.stroke.style,
            kurbo::Affine::default(),
            &self.stroke.brush,
            None,
            &kurbo::RoundedRect::from_rect(self.rect, self.radii),
        );

        fragment.fragment = frag.into();
    }

    #[inline]
    fn should_build(&self) -> bool {
        self.should_build
    }

    #[inline]
    fn set_should_build(&mut self, should_build: bool) {
        self.should_build = should_build
    }
}

pub struct VelloRectMotion {
    target_id: Entity,
    vello_rect: VelloRect,
}

impl VelloRectMotion {
    pub fn new(target_id: Entity, vello_rect: VelloRect) -> Self {
        Self {
            target_id,
            vello_rect,
        }
    }

    // =====================
    // Rect
    // =====================
    pub fn inflate(
        &mut self,
        inflation: impl Into<DVec2>,
    ) -> Action<VelloRect, kurbo::Rect, EmptyRes> {
        let inflation: DVec2 = inflation.into();

        let new_rect: kurbo::Rect = self.vello_rect.rect.inflate(inflation.x, inflation.y);

        let action: Action<VelloRect, kurbo::Rect, EmptyRes> = Action::new(
            self.target_id,
            self.vello_rect.rect,
            new_rect,
            Self::rect_interp,
        );

        self.vello_rect.rect = new_rect;

        action
    }

    pub fn percentage_expand(
        &mut self,
        expansion: impl Into<DVec2>,
        percentage: impl Into<DVec2>,
    ) -> Action<VelloRect, kurbo::Rect, EmptyRes> {
        let expansion: DVec2 = expansion.into();
        let percentage: DVec2 = percentage.into();

        let mut new_rect: kurbo::Rect = self.vello_rect.rect;
        new_rect.x0 -= expansion.x * (1.0 - percentage.x);
        new_rect.y0 -= expansion.y * (1.0 - percentage.y);
        new_rect.x1 += expansion.x * percentage.x;
        new_rect.y1 += expansion.y * percentage.y;

        let action: Action<VelloRect, kurbo::Rect, EmptyRes> = Action::new(
            self.target_id,
            self.vello_rect.rect,
            new_rect,
            Self::rect_interp,
        );

        self.vello_rect.rect = new_rect;

        action
    }

    pub fn expand_left(&mut self, expansion: f64) -> Action<VelloRect, kurbo::Rect, EmptyRes> {
        self.percentage_expand(DVec2::new(expansion, 0.0), DVec2::new(0.0, 0.0))
    }

    pub fn expand_right(&mut self, expansion: f64) -> Action<VelloRect, kurbo::Rect, EmptyRes> {
        self.percentage_expand(DVec2::new(expansion, 0.0), DVec2::new(1.0, 0.0))
    }

    pub fn expand_bottom(&mut self, expansion: f64) -> Action<VelloRect, kurbo::Rect, EmptyRes> {
        self.percentage_expand(DVec2::new(0.0, expansion), DVec2::new(0.0, 0.0))
    }

    pub fn expand_top(&mut self, expansion: f64) -> Action<VelloRect, kurbo::Rect, EmptyRes> {
        self.percentage_expand(DVec2::new(0.0, expansion), DVec2::new(0.0, 1.0))
    }

    pub fn rect_to(
        &mut self,
        new_rect: impl Into<kurbo::Rect>,
    ) -> Action<VelloRect, kurbo::Rect, EmptyRes> {
        let new_rect: kurbo::Rect = new_rect.into();

        let action: Action<VelloRect, kurbo::Rect, EmptyRes> = Action::new(
            self.target_id,
            self.vello_rect.rect,
            new_rect,
            Self::rect_interp,
        );

        self.vello_rect.rect = new_rect;

        action
    }

    fn rect_interp(
        vello_rect: &mut VelloRect,
        begin: &kurbo::Rect,
        end: &kurbo::Rect,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        vello_rect.rect = kurbo::Rect::lerp(begin, end, t);
        vello_rect.set_should_build(true);
    }

    // =====================
    // Radii
    // =====================
    pub fn radii_to(
        &mut self,
        new_radii: impl Into<kurbo::RoundedRectRadii>,
    ) -> Action<VelloRect, kurbo::RoundedRectRadii, EmptyRes> {
        let new_radii: kurbo::RoundedRectRadii = new_radii.into();

        let action: Action<VelloRect, kurbo::RoundedRectRadii, EmptyRes> = Action::new(
            self.target_id,
            self.vello_rect.radii,
            new_radii,
            Self::radii_interp,
        );

        self.vello_rect.radii = new_radii;

        action
    }

    fn radii_interp(
        vello_rect: &mut VelloRect,
        begin: &kurbo::RoundedRectRadii,
        end: &kurbo::RoundedRectRadii,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        vello_rect.radii = kurbo::RoundedRectRadii::lerp(begin, end, t);
        vello_rect.set_should_build(true);
    }

    // =====================
    // Fill brush
    // =====================
    pub fn fill_brush_to(
        &mut self,
        new_brush: impl Into<PenikoBrush>,
    ) -> Action<VelloRect, peniko::Brush, EmptyRes> {
        let new_brush: peniko::Brush = new_brush.into().0;

        let action: Action<VelloRect, peniko::Brush, EmptyRes> = Action::new(
            self.target_id,
            self.vello_rect.fill.brush.clone(),
            new_brush.clone(),
            Self::fill_brush_interp,
        );

        self.vello_rect.fill.brush = new_brush;

        action
    }

    fn fill_brush_interp(
        vello_rect: &mut VelloRect,
        begin: &peniko::Brush,
        end: &peniko::Brush,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        vello_rect.fill.brush = peniko::Brush::lerp(begin, end, t);
        vello_rect.set_should_build(true);
    }

    // =====================
    // Stroke brush
    // =====================
    pub fn stroke_brush_to(
        &mut self,
        new_brush: impl Into<PenikoBrush>,
    ) -> Action<VelloRect, peniko::Brush, EmptyRes> {
        let new_brush: peniko::Brush = new_brush.into().0;

        let action: Action<VelloRect, peniko::Brush, EmptyRes> = Action::new(
            self.target_id,
            self.vello_rect.stroke.brush.clone(),
            new_brush.clone(),
            Self::stroke_brush_interp,
        );

        self.vello_rect.stroke.brush = new_brush;

        action
    }

    fn stroke_brush_interp(
        vello_rect: &mut VelloRect,
        begin: &peniko::Brush,
        end: &peniko::Brush,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        vello_rect.stroke.brush = peniko::Brush::lerp(begin, end, t);
        vello_rect.set_should_build(true);
    }

    // =====================
    // Stroke style
    // =====================
    pub fn stroke_style_to(
        &mut self,
        new_style: impl Into<KurboStroke>,
    ) -> Action<VelloRect, kurbo::Stroke, EmptyRes> {
        let new_style: kurbo::Stroke = new_style.into().0;

        let action: Action<VelloRect, kurbo::Stroke, EmptyRes> = Action::new(
            self.target_id,
            self.vello_rect.stroke.style.clone(),
            new_style.clone(),
            Self::stroke_style_interp,
        );

        self.vello_rect.stroke.style = new_style;

        action
    }

    fn stroke_style_interp(
        vello_rect: &mut VelloRect,
        begin: &kurbo::Stroke,
        end: &kurbo::Stroke,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        vello_rect.stroke.style = kurbo::Stroke::lerp(begin, end, t);
        vello_rect.set_should_build(true);
    }
}
