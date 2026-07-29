//! Loads a [`Scene`] as a Bevy [`Asset`] from a `.mgx.ron` file.

use alloc::vec::Vec;

use bevy_asset::io::Reader;
use bevy_asset::{Asset, AssetLoader, LoadContext};
use bevy_ecs::error::BevyError;
use bevy_reflect::TypePath;
use motiongfx_scene::error::CompileError;
use motiongfx_scene::registry::SceneRegistry;
use motiongfx_scene::scene::Scene;

use crate::manager::{MotionGfxManager, TimelineId};
use crate::scene::backend::Backend;

/// A [`Scene<BevyBackend>`], loadable as a Bevy asset.
///
/// A newtype rather than a blanket impl on `Scene` itself: `Asset`/
/// `TypePath` are foreign traits and `Scene` is a foreign type from
/// this crate's perspective, so the orphan rule requires a local
/// wrapper.
#[derive(Asset, TypePath)]
pub struct MotionGfxScene(pub Scene<Backend>);

impl MotionGfxScene {
    /// Compiles this scene into a `BevyTimeline` through
    /// `scene_registry`, then registers it on `motiongfx` exactly
    /// like [`MotionGfxManager::add_timeline`]. This scene's subjects
    /// must already be materialized (see
    /// [`spawn_scene`](crate::scene::spawn_scene)) so their
    /// [`SceneId`](crate::scene::id::SceneId)s resolve through the
    /// world's [`SceneEntityMap`](crate::scene::id::SceneEntityMap).
    pub fn compile(
        &self,
        scene_registry: &SceneRegistry<Backend>,
        motiongfx: &mut MotionGfxManager,
    ) -> Result<TimelineId, CompileError<Backend>> {
        let timeline = motiongfx_scene::compile::compile(
            &self.0,
            scene_registry,
            motiongfx.registry_mut(),
        )?;
        Ok(motiongfx.add_timeline(timeline))
    }
}

#[derive(Default, TypePath)]
pub struct SceneAssetLoader;

impl AssetLoader for SceneAssetLoader {
    type Asset = MotionGfxScene;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let scene = ron::de::from_bytes(&bytes)?;
        Ok(MotionGfxScene(scene))
    }

    fn extensions(&self) -> &[&str] {
        &["mgx.ron"]
    }
}
