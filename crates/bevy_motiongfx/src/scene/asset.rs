//! Loads a [`Scene`] as a Bevy [`Asset`] from a `.mgx.ron` file.

use alloc::vec::Vec;

use bevy_asset::io::Reader;
use bevy_asset::{Asset, AssetLoader, LoadContext};
use bevy_ecs::error::BevyError;
use bevy_ecs::world::World;
use bevy_reflect::TypePath;
use motiongfx_scene::error::CompileError;
use motiongfx_scene::scene::Scene;

use crate::manager::{MotionGfxManager, TimelineId};
use crate::scene::backend::{Backend, BackendRegistry};
use crate::world::BevyWorld;

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
    /// must already be materialized - however the app chooses to do
    /// that - so their [`EntityUid`](crate::scene::id::EntityUid)s
    /// resolve through the world's
    /// [`SceneUidMap`](crate::scene::id::SceneUidMap).
    pub fn compile(
        &self,
        scene_registry: &BackendRegistry,
        motiongfx: &mut MotionGfxManager,
    ) -> Result<TimelineId, CompileError<Backend>> {
        let timeline = self
            .0
            .compile(scene_registry, motiongfx.registry_mut())?;
        Ok(motiongfx.add_timeline(timeline))
    }

    /// Writes this scene's initial values onto its already-materialized
    /// subjects.
    ///
    /// Baking reads each track's starting value off the world, and that
    /// happens in [`MotionGfxSystems::Sample`](crate::MotionGfxSystems::Sample),
    /// so call this any time before then - alongside [`Self::compile`]
    /// is the natural place. Skip it and the animation starts from
    /// whatever the entities already held.
    pub fn stage(
        &self,
        scene_registry: &BackendRegistry,
        world: &mut World,
    ) -> Result<(), CompileError<Backend>> {
        self.0.stage(scene_registry, BevyWorld::from_mut(world))
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
