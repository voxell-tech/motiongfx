#![no_std]

use bevy_app::prelude::*;
use motiongfx_engine::prelude::*;

pub struct MotionGfxCommonPlugin;

impl Plugin for MotionGfxCommonPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "transform")]
        {
            use bevy_transform::components::Transform;

            register_fields!(
                app.register_component_field(),
                Transform,
                (
                    translation(x, y, z),
                    scale(x, y, z),
                    rotation(x, y, z, w),
                )
            );
        }

        #[cfg(feature = "sprite")]
        {
            use bevy_sprite::prelude::*;

            register_fields!(
                app.register_component_field(),
                Sprite,
                (
                    image,
                    color,
                    flip_x,
                    flip_y,
                    custom_size,
                    rect,
                    anchor,
                    image_mode,
                )
            );
            register_fields!(
                app.register_asset_field::<MeshMaterial2d<_>>(),
                ColorMaterial,
                (color, alpha_mode, uv_transform, texture,)
            );
        }

        #[cfg(feature = "pbr")]
        {
            use bevy_pbr::prelude::*;

            register_fields!(
                app.register_asset_field::<MeshMaterial3d<_>>(),
                StandardMaterial,
                (
                    base_color,
                    emissive,
                    perceptual_roughness,
                    metallic,
                    reflectance,
                    specular_tint,
                    diffuse_transmission,
                    specular_transmission,
                    thickness,
                    ior,
                    attenuation_distance,
                    attenuation_color,
                )
            );
        }
    }
}
