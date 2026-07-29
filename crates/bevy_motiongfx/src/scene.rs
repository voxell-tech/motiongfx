//! Serializable scenes for Bevy: a [`SceneBackend`](motiongfx_scene::backend::SceneBackend)
//! implementation, plus loading one as a Bevy [`Asset`](bevy_asset::Asset).
//! Gated behind the `scene` feature.

use alloc::vec::Vec;

use asset::{MotionGfxScene, SceneAssetLoader};
use bevy_app::prelude::*;
use bevy_asset::AssetApp as _;
use bevy_ecs::prelude::*;
use bevy_transform::components::Transform;
use id::{EntityUid, SceneUidMap, on_add_entity_uid, on_remove_entity_uid};

pub mod asset;
pub mod backend;
pub mod id;
pub mod value_pool;

pub struct MotionGfxScenePlugin;

impl Plugin for MotionGfxScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneUidMap>()
            .init_asset::<MotionGfxScene>()
            .register_asset_loader(SceneAssetLoader)
            .add_observer(on_add_entity_uid)
            .add_observer(on_remove_entity_uid);
    }
}

/// Spawns an entity with a default [`Transform`] plus its
/// [`EntityUid`] component for each subject in `scene`'s stage,
/// returning each subject's `(EntityUid, Entity)` pair so callers can
/// attach further components immediately - `Commands::spawn` reserves
/// the `Entity` synchronously, but [`SceneUidMap`] only sees it once
/// the spawn command is actually applied (see `on_add_entity_uid`), so
/// it isn't queryable within the same system yet.
///
/// **Stub**: only spawns a bare `Transform` - meshes, materials, and
/// every other bit of an entity's static composition aren't part of
/// this format and still need to be attached by hand (see
/// `scene_demo`). Meant to be replaced once entity composition is
/// loaded through a real Bevy scene serializer (`DynamicScene`/
/// `.scn.ron` + reflection), with this crate's format staying scoped
/// to animation only.
pub fn spawn_scene(
    commands: &mut Commands,
    scene: &MotionGfxScene,
) -> Vec<(EntityUid, Entity)> {
    scene
        .0
        .stage
        .subjects
        .iter()
        .map(|subject| {
            let entity = commands
                .spawn((Transform::default(), subject.id))
                .id();
            (subject.id, entity)
        })
        .collect()
}
