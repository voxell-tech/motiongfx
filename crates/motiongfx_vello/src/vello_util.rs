use bevy_vello_renderer::{vello::peniko, vello_svg::usvg};

pub fn paint_to_brush(paint: &usvg::Paint, opacity: usvg::Opacity) -> Option<peniko::Brush> {
    match paint {
        usvg::Paint::Color(color) => Some(peniko::Brush::Solid(peniko::Color::rgba8(
            color.red,
            color.green,
            color.blue,
            opacity.to_u8(),
        ))),
        usvg::Paint::LinearGradient(_) => None,
        usvg::Paint::RadialGradient(_) => None,
        usvg::Paint::Pattern(_) => None,
    }
}
