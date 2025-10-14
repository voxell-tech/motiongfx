use bevy_render::prelude::*;
use bevy_vello_renderer::vello::{kurbo, peniko};

pub struct PenikoColor(pub peniko::Color);

impl From<peniko::Color> for PenikoColor {
    #[inline]
    fn from(value: peniko::Color) -> Self {
        PenikoColor(value)
    }
}

impl From<Color> for PenikoColor {
    #[inline]
    fn from(value: Color) -> Self {
        PenikoColor(peniko::Color::rgba(
            value.r() as f64,
            value.g() as f64,
            value.b() as f64,
            value.a() as f64,
        ))
    }
}

#[derive(Default)]
pub struct PenikoBrush(pub peniko::Brush);

impl From<peniko::Brush> for PenikoBrush {
    #[inline]
    fn from(value: peniko::Brush) -> Self {
        PenikoBrush(value)
    }
}

impl From<Color> for PenikoBrush {
    #[inline]
    fn from(value: Color) -> Self {
        PenikoBrush(peniko::Brush::Solid(PenikoColor::from(value).0))
    }
}

pub struct KurboStroke(pub kurbo::Stroke);

impl From<kurbo::Stroke> for KurboStroke {
    #[inline]
    fn from(value: kurbo::Stroke) -> Self {
        KurboStroke(value)
    }
}

impl From<f32> for KurboStroke {
    #[inline]
    fn from(value: f32) -> Self {
        KurboStroke::from(value as f64)
    }
}

impl From<f64> for KurboStroke {
    #[inline]
    fn from(value: f64) -> Self {
        KurboStroke(kurbo::Stroke::new(value))
    }
}
