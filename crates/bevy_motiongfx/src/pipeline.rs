use bevy_ecs::component::Mutable;
use bevy_ecs::prelude::*;
use motiongfx::pipeline::{bake, sample};
use motiongfx::prelude::*;

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
            .map(f)
    }
}

pub type BevyPipelineRegistry = PipelineRegistry<BevyWorld>;
pub type BevyPipeline = Pipeline<BevyWorld>;

pub trait PipelineRegistryExt {
    fn register_component<S, T>(&mut self) -> PipelineKey
    where
        S: Component<Mutability = Mutable>,
        T: Clone + ThreadSafe;

    #[cfg(feature = "asset")]
    fn register_asset<S, T>(&mut self) -> PipelineKey
    where
        S: bevy_asset::Asset,
        T: Clone + ThreadSafe;
}

impl PipelineRegistryExt for BevyPipelineRegistry {
    fn register_component<S, T>(&mut self) -> PipelineKey
    where
        S: Component<Mutability = Mutable>,
        T: Clone + ThreadSafe,
    {
        let key = PipelineKey::new::<Entity, S, T>();

        self.register_unchecked(
            key,
            BevyPipeline::new(
                bake::<_, _, S, T>,
                sample::<_, _, S, T>,
            ),
        );

        key
    }

    #[cfg(feature = "asset")]
    fn register_asset<S, T>(&mut self) -> PipelineKey
    where
        S: bevy_asset::Asset,
        T: Clone + ThreadSafe,
    {
        use bevy_asset::UntypedAssetId;

        let key = PipelineKey::new::<UntypedAssetId, S, T>();

        self.register_unchecked(
            key,
            BevyPipeline::new(
                bake::<_, _, S, T>,
                sample::<_, _, S, T>,
            ),
        );

        key
    }
}
