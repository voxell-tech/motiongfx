//! Cursor-glow driving and button hover/press material swaps.

use bevy::picking::hover::Hovered;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::ui::{Pressed, UiGlobalTransform};
use bevy::ui_render::prelude::MaterialNode;
use bevy::ui_widgets::Button;

use super::material::GlassMaterial;
use super::preset::GlassAssets;

/// Push the cursor (physical px) into every glass material and enable
/// glow only for materials whose node contains it.
pub(super) fn update_glow(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    q_surfaces: Query<(
        &MaterialNode<GlassMaterial>,
        &ComputedNode,
        &UiGlobalTransform,
    )>,
    mut materials: ResMut<Assets<GlassMaterial>>,
    mut last: Local<(Vec2, HashSet<AssetId<GlassMaterial>>)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    // Park the glow far away when the cursor leaves the window.
    let pos = window
        .cursor_position()
        .map(|p| p * window.scale_factor())
        .unwrap_or(Vec2::splat(-1.0e6));

    // Materials with the cursor inside one of their nodes.
    let hovered: HashSet<AssetId<GlassMaterial>> = q_surfaces
        .iter()
        .filter(|(_, computed, transform)| {
            let (_, _, center) =
                transform.to_scale_angle_translation();
            Rect::from_center_size(center, computed.size())
                .contains(pos)
        })
        .map(|(node, _, _)| node.0.id())
        .collect();

    if last.0 == pos && last.1 == hovered {
        return;
    }
    *last = (pos, hovered.clone());

    let radius = 160.0 * window.scale_factor();
    for (id, material) in materials.iter_mut() {
        // Preserve `glow.w`: the per-material radius scale.
        material.glow.x = pos.x;
        material.glow.y = pos.y;
        material.glow.z = radius;
        material.params.w = if hovered.contains(&id) {
            material.base_glow
        } else {
            0.0
        };
    }
}

/// Swap button materials on hover / press.
pub(super) fn update_glass_buttons(
    assets: Res<GlassAssets>,
    mut q_buttons: Query<
        (&Hovered, Has<Pressed>, &mut MaterialNode<GlassMaterial>),
        With<Button>,
    >,
) {
    for (hovered, pressed, mut node) in &mut q_buttons {
        let want = if pressed {
            &assets.button_pressed
        } else if hovered.get() {
            &assets.button_hover
        } else {
            &assets.button
        };
        if node.0 != *want {
            node.0 = want.clone();
        }
    }
}
