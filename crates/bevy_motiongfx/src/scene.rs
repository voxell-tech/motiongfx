//! Serializable scenes for Bevy: a [`SceneBackend`](motiongfx_scene::backend::SceneBackend)
//! implementation, plus loading one as a Bevy [`Asset`](bevy_asset::Asset).
//! Gated behind the `scene` feature.

use asset::{MotionGfxScene, SceneAssetLoader};
use bevy_app::prelude::*;
use bevy_asset::AssetApp as _;
use id::{EntityUid, SceneUidMap, on_add_entity_uid, on_remove_entity_uid};

pub mod asset;
pub mod backend;
pub mod id;
pub mod value_pool;

pub struct MotionGfxScenePlugin;

impl Plugin for MotionGfxScenePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<EntityUid>()
            .init_resource::<SceneUidMap>()
            .init_asset::<MotionGfxScene>()
            .register_asset_loader(SceneAssetLoader)
            .add_observer(on_add_entity_uid)
            .add_observer(on_remove_entity_uid);
    }
}
