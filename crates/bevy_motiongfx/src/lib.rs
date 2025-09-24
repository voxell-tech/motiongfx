#![no_std]

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use motiongfx::prelude::*;

pub mod interpolation;
pub mod registry;

pub mod prelude {
    pub use crate::interpolation::{
        ActionInterpTimelineExt, Interpolation,
    };
    pub use crate::register_fields;
    pub use crate::registry::FieldPathRegisterAppExt;
    pub use motiongfx::prelude::*;
}

pub struct BevyMotionGfxPlugin;

impl Plugin for BevyMotionGfxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FieldAccessorRegistry>()
            .init_resource::<PipelineRegistry>();

        app.configure_sets(
            PostUpdate,
            (
                MotionGfxSet::QueueAction,
                #[cfg(not(feature = "transform"))]
                MotionGfxSet::Sample,
                #[cfg(feature = "transform")]
                MotionGfxSet::Sample
                    .before(bevy_transform::TransformSystem::TransformPropagate),
            )
                .chain(),
        );

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
                (color, alpha_mode, uv_transform, texture)
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

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MotionGfxSet {
    /// Queue actions that will be sampled by marking them.
    QueueAction,
    /// Sample keyframes and applies the value.
    Sample,
}
