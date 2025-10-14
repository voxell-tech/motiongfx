use bevy_ecs::prelude::*;
use bevy_render::prelude::*;
use bevy_utils::prelude::*;
use bevy_vello_renderer::vello::{self, kurbo, peniko};
use motiongfx_core::prelude::*;

use crate::convert::*;
use crate::vello_vector::VelloBuilder;

#[derive(Component, Clone)]
pub struct FillStyle {
    pub style: peniko::Fill,
    pub brush: peniko::Brush,
    pub transform: kurbo::Affine,
    built: bool,
}

impl FillStyle {
    pub fn new(
        style: peniko::Fill,
        brush: impl Into<PenikoBrush>,
        transform: kurbo::Affine,
    ) -> Self {
        let brush: peniko::Brush = brush.into().0;

        Self {
            style,
            brush,
            transform,
            ..default()
        }
    }

    #[inline]
    pub fn from_brush(brush: impl Into<PenikoBrush>) -> Self {
        Self::default().with_brush(brush)
    }

    #[inline]
    pub fn with_style(mut self, style: peniko::Fill) -> Self {
        self.style = style;
        self
    }

    #[inline]
    pub fn with_brush(mut self, brush: impl Into<PenikoBrush>) -> Self {
        self.brush = brush.into().0;
        self
    }

    #[inline]
    pub fn build(&self, scene: &mut vello::Scene, shape: &impl kurbo::Shape) {
        scene.fill(
            self.style,
            kurbo::Affine::IDENTITY,
            &self.brush,
            Some(self.transform),
            shape,
        );
    }
}

impl VelloBuilder for FillStyle {
    #[inline]
    fn is_built(&self) -> bool {
        self.built
    }

    #[inline]
    fn set_built(&mut self, built: bool) {
        self.built = built
    }
}

impl Default for FillStyle {
    fn default() -> Self {
        Self {
            style: peniko::Fill::NonZero,
            brush: peniko::Brush::Solid(peniko::Color::WHITE_SMOKE),
            transform: kurbo::Affine::IDENTITY,
            built: false,
        }
    }
}

impl From<Color> for FillStyle {
    fn from(value: Color) -> Self {
        Self {
            brush: peniko::Brush::Solid(peniko::Color::rgba(
                value.r() as f64,
                value.g() as f64,
                value.b() as f64,
                value.a() as f64,
            )),
            ..default()
        }
    }
}

pub struct FillStyleMotion {
    target_id: Entity,
    fill: FillStyle,
}

impl FillStyleMotion {
    pub fn new(target_id: Entity, fill: FillStyle) -> Self {
        Self { target_id, fill }
    }

    pub fn brush_to(
        &mut self,
        new_brush: impl Into<PenikoBrush>,
    ) -> Action<FillStyle, peniko::Brush, EmptyRes> {
        let new_brush: peniko::Brush = new_brush.into().0;

        let action: Action<FillStyle, peniko::Brush, EmptyRes> = Action::new(
            self.target_id,
            self.fill.brush.clone(),
            new_brush.clone(),
            Self::brush_interp,
        );

        self.fill.brush = new_brush;

        action
    }

    fn brush_interp(
        fill: &mut FillStyle,
        begin: &peniko::Brush,
        end: &peniko::Brush,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        fill.brush = peniko::Brush::lerp(begin, end, t);
        fill.set_built(false);
    }
}
