use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_math::prelude::*;
use bevy_pbr::prelude::*;
use bevy_transform::prelude::*;
use motiongfx_core::prelude::*;

pub mod standard_material;
pub mod transform;

pub mod prelude {
    pub use crate::{
        standard_material::{BaseColor, Emissive},
        transform::{Rotation, Scale, Translation},
        MotionGfxBevy,
    };
}

pub struct MotionGfxBevy;

impl Plugin for MotionGfxBevy {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            ((
                sequence_player_system::<Transform, Vec3, EmptyRes>,
                sequence_player_system::<Transform, Quat, EmptyRes>,
                sequence_player_system::<Handle<StandardMaterial>, Vec4, Assets<StandardMaterial>>,
            ),),
        );
    }
}
