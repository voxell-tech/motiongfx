//! The delay gap's tiled hatch texture.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::render_resource::{Extent3d, TextureDimension};

/// A small tileable diagonal hatch, for
/// [`TimelineGap`](moxie_ui::elements::TimelineGap).
#[derive(Resource)]
pub(crate) struct DelayPattern(pub(crate) Handle<Image>);

impl FromWorld for DelayPattern {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        Self(images.add(hatch()))
    }
}

/// An 8x8 diagonal hatch: opaque on the stripe, transparent between -
/// [`TimelineGap`](moxie_ui::elements::TimelineGap) tints and dims it
/// with its own `color`.
fn hatch() -> Image {
    const SIZE: u32 = 8;
    const STRIPE: u32 = 2;

    let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let on = (x + y) % (STRIPE * 2) < STRIPE;
            data.extend_from_slice(&[
                255,
                255,
                255,
                if on { 255 } else { 0 },
            ]);
        }
    }

    Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
