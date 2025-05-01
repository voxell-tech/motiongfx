use bevy::prelude::*;
use motiongfx_core::{prelude::*, UpdateSequenceSet};

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
        app.add_systems(
            Update,
            (
                update_component::<Transform, Vec3>,
                update_component::<Transform, Quat>,
                update_component::<Transform, f32>,
                update_component::<Sprite, Color>,
                update_component::<Sprite, f32>,
                update_asset::<MeshMaterial3d<StandardMaterial>, Color>,
                update_asset::<MeshMaterial3d<StandardMaterial>, LinearRgba>,
                update_asset::<MeshMaterial3d<StandardMaterial>, f32>,
                update_asset::<MeshMaterial2d<ColorMaterial>, Color>,
                update_asset::<MeshMaterial2d<ColorMaterial>, f32>,
            )
                .in_set(UpdateSequenceSet),
        );
    }
}
