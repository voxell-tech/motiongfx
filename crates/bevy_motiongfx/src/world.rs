use bevy_ecs::component::Mutable;
use bevy_ecs::prelude::*;
use motiongfx::prelude::*;

#[cfg(feature = "scene")]
use crate::scene::id::{EntityUid, SceneUidMap};

/// Newtype wrapper around [`World`] that is local to this crate,
/// allowing [`SubjectSource`] impls without violating the orphan rule.
#[repr(transparent)]
pub struct BevyWorld(pub World);

impl BevyWorld {
    pub fn from_ref(world: &World) -> &Self {
        // SAFETY: `BevyWorld` is repr(transparent) over `World`.
        unsafe { &*(world as *const World as *const Self) }
    }

    pub fn from_mut(world: &mut World) -> &mut Self {
        // SAFETY: `BevyWorld` is repr(transparent) over `World`.
        unsafe { &mut *(world as *mut World as *mut Self) }
    }
}

impl<S: Component<Mutability = Mutable>> SubjectSource<Entity, S>
    for BevyWorld
{
    fn get_source(&self, id: Entity) -> Option<&S> {
        self.0.get::<S>(id)
    }

    fn apply_source<R>(
        &mut self,
        id: Entity,
        f: impl FnOnce(&mut S) -> R,
    ) -> Option<R> {
        self.0.get_mut::<S>(id).map(|mut m| f(m.as_mut()))
    }
}

/// Resolves `id` to an `Entity` through the world's [`SceneUidMap`]
/// resource, then delegates to the `Entity`-keyed impl above. `Entity`
/// is generational and gets reassigned on every scene load, so nothing
/// outside `SceneUidMap` may assume it is stable across a save/reload
/// cycle - only the `EntityUid` is.
#[cfg(feature = "scene")]
impl<S: Component<Mutability = Mutable>> SubjectSource<EntityUid, S>
    for BevyWorld
{
    fn get_source(&self, id: EntityUid) -> Option<&S> {
        let entity =
            self.0.get_resource::<SceneUidMap>()?.entity(id)?;
        self.0.get::<S>(entity)
    }

    fn apply_source<R>(
        &mut self,
        id: EntityUid,
        f: impl FnOnce(&mut S) -> R,
    ) -> Option<R> {
        let entity =
            self.0.get_resource::<SceneUidMap>()?.entity(id)?;
        self.0.get_mut::<S>(entity).map(|mut m| f(m.as_mut()))
    }
}

#[cfg(feature = "asset")]
impl<S: bevy_asset::Asset>
    SubjectSource<bevy_asset::UntypedAssetId, S> for BevyWorld
{
    fn get_source(
        &self,
        id: bevy_asset::UntypedAssetId,
    ) -> Option<&S> {
        self.0
            .get_resource::<bevy_asset::Assets<S>>()?
            .get(id.typed::<S>())
    }

    fn apply_source<R>(
        &mut self,
        id: bevy_asset::UntypedAssetId,
        f: impl FnOnce(&mut S) -> R,
    ) -> Option<R> {
        self.0
            .get_resource_mut::<bevy_asset::Assets<S>>()?
            .into_inner()
            .get_mut(id.typed::<S>())
            .map(|asset| f(asset.into_inner()))
    }
}

pub type BevyTimeline = Timeline<BevyWorld>;
pub type BevyTimelineBuilder<'a> = TimelineBuilder<'a, BevyWorld>;
