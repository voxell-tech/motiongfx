use bevy::{
    ecs::system::{EntityCommand, EntityCommands},
    prelude::*,
};
use motiongfx_core::{prelude::*, UpdateSequenceSet};

pub mod motion;

pub mod prelude {
    pub use crate::{
        motion::{
            standard_material_motion::StandardMaterialMotion, transform_motion::TransformMotion,
        },
        AddNewAssetCommandExt, MotionGfxCommonPlugin,
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
                update_asset::<StandardMaterial, Color>,
                update_asset::<StandardMaterial, LinearRgba>,
                update_asset::<StandardMaterial, f32>,
                update_asset::<ColorMaterial, Color>,
                update_asset::<ColorMaterial, f32>,
            )
                .in_set(UpdateSequenceSet),
        );
    }
}

pub trait AddNewAssetCommandExt<A: Asset> {
    /// Adds a new asset and attach the handle to this entity.
    fn add_new_asset(&mut self, asset: A) -> &mut Self;
}

impl<A: Asset> AddNewAssetCommandExt<A> for EntityCommands<'_> {
    fn add_new_asset(&mut self, asset: A) -> &mut Self {
        self.add(AddNewAssetCommand(asset))
    }
}

pub struct AddNewAssetCommand<A: Asset>(A);

impl<A: Asset> EntityCommand for AddNewAssetCommand<A> {
    fn apply(self, id: Entity, world: &mut World) {
        let mut materials = world.get_resource_mut::<Assets<A>>().unwrap_or_else(|| {
            panic!(
                "Assets<{}> resource not initialized.",
                A::type_ident().unwrap()
            )
        });

        let material = materials.add(self.0);

        world.entity_mut(id).insert(material);
    }
}
