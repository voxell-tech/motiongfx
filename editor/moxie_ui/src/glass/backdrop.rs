//! The refraction/frost backdrop source.

use bevy::ecs::query::QueryState;
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use super::material::GlassMaterial;
use crate::reactive::BevyUi;

/// Marks a UI node displaying a backdrop image (e.g. the editor's
/// scene preview) that glass panes frost where they overlap it.
/// Carries the image so the glass module stays app-agnostic.
#[derive(Component)]
pub struct GlassBackdrop(pub Handle<Image>);

/// Mirror the backdrop's on-screen rect and image into every glass
/// material.
///
/// The write lands on assets, not on the bound node, so this hangs off
/// whatever node declares it purely for lifetime: a binding's signal
/// and its write target are independent.
pub fn bind_backdrop(ui: &mut BevyUi) {
    let mut backdrops: Option<
        QueryState<(Entity, &'static GlassBackdrop)>,
    > = None;
    let mut seen_rect = Vec4::ZERO;
    let mut seen_image: Option<AssetId<Image>> = None;

    ui.empty_node().bind_raw(
        move |world, _| {
            let backdrops = match &mut backdrops {
                Some(query) => query,
                slot => match QueryState::try_new(world) {
                    Some(query) => slot.insert(query),
                    None => return false,
                },
            };
            backdrops.update_archetypes(world);
            // Track the image id too: a new handle with an unchanged
            // rect must still repropagate to the glass materials.
            let current = backdrops.iter_manual(world).next().map(
                |(entity, backdrop)| {
                    (backdrop_rect(world, entity), backdrop.0.id())
                },
            );
            let rect = current.map_or(Vec4::ZERO, |(r, _)| r);
            let image = current.map(|(_, id)| id);
            let changed = seen_rect != rect || seen_image != image;
            seen_rect = rect;
            seen_image = image;
            changed
        },
        |world, _| {
            let source = world
                .query::<(Entity, &GlassBackdrop)>()
                .iter(world)
                .next()
                .map(|(entity, backdrop)| {
                    (entity, backdrop.0.clone())
                });
            let rect =
                source.as_ref().map_or(Vec4::ZERO, |(entity, _)| {
                    backdrop_rect(world, *entity)
                });
            let image = source.map(|(_, image)| image);

            let mut materials =
                world.resource_mut::<Assets<GlassMaterial>>();
            for (_, material) in materials.iter_mut() {
                material.backdrop_rect = rect;
                material.backdrop = image.clone();
            }
        },
    );
}

/// The backdrop node's on-screen rect in physical px, matching the
/// fragment shader's framebuffer coordinates.
fn backdrop_rect(world: &World, entity: Entity) -> Vec4 {
    let Some(computed) = world.get::<ComputedNode>(entity) else {
        return Vec4::ZERO;
    };
    let Some(transform) = world.get::<UiGlobalTransform>(entity)
    else {
        return Vec4::ZERO;
    };
    let size = computed.size();
    let (_, _, center) = transform.to_scale_angle_translation();
    let min = center.trunc() - size * 0.5;
    Vec4::new(min.x, min.y, size.x, size.y)
}
