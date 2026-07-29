//! Serializable scenes for Bevy: a [`SceneBackend`](motiongfx_scene::backend::SceneBackend)
//! implementation, plus loading one as a Bevy [`Asset`](bevy_asset::Asset).
//! Gated behind the `scene` feature.

use asset::{MotionGfxScene, SceneAssetLoader};
use bevy_app::prelude::*;
use bevy_asset::AssetApp as _;
use bevy_ecs::prelude::*;
use bevy_transform::components::Transform;
use id::SceneEntityMap;

pub mod asset;
pub mod backend;
pub mod id;
pub mod value_pool;

pub struct MotionGfxScenePlugin;

impl Plugin for MotionGfxScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneEntityMap>()
            .init_asset::<MotionGfxScene>()
            .register_asset_loader(SceneAssetLoader);
    }
}

/// Spawns an entity with a default [`Transform`] for each subject in
/// `scene`'s stage, recording the mapping in `entity_map` so
/// `BevyBackend`'s [`SceneId`](id::SceneId)-keyed
/// [`SubjectSource`](motiongfx::world::SubjectSource) impls (see
/// `crate::world`) can resolve them once the scene is compiled and
/// played.
pub fn spawn_scene(
    commands: &mut Commands,
    entity_map: &mut SceneEntityMap,
    scene: &MotionGfxScene,
) {
    for subject in &scene.0.stage.subjects {
        let entity = commands.spawn(Transform::default()).id();
        entity_map.insert(subject.id, entity);
    }
}
