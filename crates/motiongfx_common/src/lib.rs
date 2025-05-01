use bevy::prelude::*;
use motiongfx_core::prelude::*;

pub mod motion;

pub mod prelude {
    pub use crate::{
        motion::{
            standard_material_motion::StandardMaterialMotion, transform_motion::TransformMotion,
        },
        MotionGfxCommonPlugin,
    };
}

pub struct MotionGfxCommonPlugin;

impl Plugin for MotionGfxCommonPlugin {
    fn build(&self, app: &mut App) {
        app.animate_component::<Transform, Vec3>()
            .animate_component::<Transform, Quat>()
            .animate_component::<Transform, f32>()
            .animate_component::<Sprite, Color>()
            .animate_component::<Sprite, f32>()
            .animate_asset::<MeshMaterial3d<StandardMaterial>, Color>()
            .animate_asset::<MeshMaterial3d<StandardMaterial>, LinearRgba>()
            .animate_asset::<MeshMaterial3d<StandardMaterial>, f32>()
            .animate_asset::<MeshMaterial2d<ColorMaterial>, Color>()
            .animate_asset::<MeshMaterial2d<ColorMaterial>, f32>();
    }
}
