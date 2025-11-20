use bevy_ecs::component::Mutable;
use bevy_ecs::prelude::*;
use motiongfx::prelude::*;

pub type WorldPipelineRegistry = PipelineRegistry<World>;
pub type WorldPipeline = Pipeline<World>;

pub fn bake_component_actions<S, T>(world: &World, ctx: BakeCtx)
where
    S: Component,
    T: Clone + ThreadSafe,
{
    ctx.bake::<Entity, S, T>(|entity| world.get::<S>(entity));
}

pub fn sample_component_actions<S, T>(
    world: &mut World,
    ctx: SampleCtx,
) where
    S: Component<Mutability = Mutable>,
    T: Clone + ThreadSafe,
{
    ctx.sample::<Entity, S, T>(|entity, target, accessor| {
        if let Some(mut source) = world.get_mut::<S>(entity) {
            *accessor.get_mut(&mut source) = target;
        }
    });
}

#[cfg(feature = "asset")]
pub fn bake_asset_actions<S, T>(world: &World, ctx: BakeCtx)
where
    S: bevy_asset::Asset,
    T: Clone + ThreadSafe,
{
    use bevy_asset::Assets;
    use bevy_asset::UntypedAssetId;

    let Some(assets) = world.get_resource::<Assets<S>>() else {
        return;
    };

    ctx.bake::<UntypedAssetId, S, T>(|asset_id| {
        assets.get(asset_id.typed::<S>())
    });
}

#[cfg(feature = "asset")]
pub fn sample_asset_actions<S, T>(world: &mut World, ctx: SampleCtx)
where
    S: bevy_asset::Asset,
    T: Clone + ThreadSafe,
{
    use bevy_asset::Assets;
    use bevy_asset::UntypedAssetId;

    let Some(mut assets) = world.get_resource_mut::<Assets<S>>()
    else {
        return;
    };

    ctx.sample::<UntypedAssetId, S, T>(
        |asset_id, target, accessor| {
            if let Some(source) =
                assets.get_mut(asset_id.typed::<S>())
            {
                *accessor.get_mut(source) = target;
            }
        },
    );
}

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

impl PipelineRegistryExt for WorldPipelineRegistry {
    fn register_component<S, T>(&mut self) -> PipelineKey
    where
        S: Component<Mutability = Mutable>,
        T: Clone + ThreadSafe,
    {
        let key = PipelineKey::new::<Entity, S, T>();

        self.register_unchecked(
            key,
            WorldPipeline::new(
                bake_component_actions::<S, T>,
                sample_component_actions::<S, T>,
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
            WorldPipeline::new(
                bake_asset_actions::<S, T>,
                sample_asset_actions::<S, T>,
            ),
        );

        key
    }
}
