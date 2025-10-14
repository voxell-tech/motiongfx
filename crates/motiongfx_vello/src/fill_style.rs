use bevy_ecs::prelude::*;
use bevy_render::prelude::*;
use bevy_utils::prelude::*;
use bevy_vello_renderer::vello::{self, kurbo, peniko};
use motiongfx_core::prelude::*;

use crate::convert::*;
use crate::vello_vector::VelloBuilder;

#[derive(VelloBuilder, Component, Clone)]
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

impl Default for FillStyle {
    fn default() -> Self {
        Self {
            style: peniko::Fill::NonZero,
            brush: peniko::Brush::Solid(peniko::Color::rgb8(252, 252, 250)),
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

    pub fn alpha_to(&mut self, new_alpha: f32) -> Action<FillStyle, f32, EmptyRes> {
        let mut alpha = 0.0;

        match &mut self.fill.brush {
            peniko::Brush::Solid(color) => {
                alpha = (color.a / 255) as f32;
                color.a = (new_alpha * 255.0) as u8;
            }
            peniko::Brush::Gradient(grad) => {
                if grad.stops.len() > 0 {
                    alpha = (grad.stops[0].color.a / 255) as f32;
                }
                for stop in &mut grad.stops {
                    stop.color.a = (new_alpha * 255.0) as u8;
                }
            }
            peniko::Brush::Image(_) => {}
        }

        Action::new(self.target_id, alpha, new_alpha, Self::alpha_interp)
    }

    fn alpha_interp(
        fill: &mut FillStyle,
        begin: &f32,
        end: &f32,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        let a = f32::lerp(begin, end, t);
        match &mut fill.brush {
            peniko::Brush::Solid(color) => color.a = (a * 255.0) as u8,
            peniko::Brush::Gradient(grad) => {
                for stop in &mut grad.stops {
                    stop.color.a = (a * 255.0) as u8;
                }
            }
            peniko::Brush::Image(_) => {}
        }

        fill.set_built(false);
    }

    pub fn get_brush(&self) -> &peniko::Brush {
        &self.fill.brush
    }
}
