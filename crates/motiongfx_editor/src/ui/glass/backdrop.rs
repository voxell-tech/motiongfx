//! The refraction/frost backdrop source.

use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use super::material::GlassMaterial;

/// Marks a UI node displaying a backdrop image (e.g. the editor's
/// scene preview) that glass panes frost where they overlap it.
/// Carries the image so the glass module stays app-agnostic.
#[derive(Component)]
pub struct GlassBackdrop(pub Handle<Image>);

/// Mirror the backdrop node's on-screen rect (physical px, matching
/// the fragment shader's framebuffer coordinates) and image into
/// every glass material.
pub(super) fn sync_backdrop(
    q_backdrop: Query<(
        &GlassBackdrop,
        &ComputedNode,
        &UiGlobalTransform,
    )>,
    mut materials: ResMut<Assets<GlassMaterial>>,
    mut last: Local<Vec4>,
) {
    let source = q_backdrop.single().ok();
    let rect =
        source.map_or(Vec4::ZERO, |(_, computed, transform)| {
            let size = computed.size();
            let (_, _, center) =
                transform.to_scale_angle_translation();
            let min = center.trunc() - size * 0.5;
            Vec4::new(min.x, min.y, size.x, size.y)
        });
    if *last == rect {
        return;
    }
    *last = rect;

    let image = source.map(|(backdrop, _, _)| backdrop.0.clone());
    for (_, material) in materials.iter_mut() {
        material.backdrop_rect = rect;
        material.backdrop = image.clone();
    }
}
