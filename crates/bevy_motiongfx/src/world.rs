use bevy_ecs::component::Mutable;
use bevy_ecs::prelude::*;
use motiongfx::prelude::*;

pub struct Bevy;

impl<S: Component<Mutability = Mutable>>
    SubjectSource<Bevy, Entity, S> for World
{
    fn get_source(&self, id: Entity) -> Option<&S> {
        self.get::<S>(id)
    }

    fn apply_source<R>(
        &mut self,
        id: Entity,
        f: impl FnOnce(&mut S) -> R,
    ) -> Option<R> {
        self.get_mut::<S>(id).map(|mut m| f(m.as_mut()))
    }
}

#[cfg(feature = "asset")]
impl<S: bevy_asset::Asset>
    SubjectSource<Bevy, bevy_asset::UntypedAssetId, S> for World
{
    fn get_source(
        &self,
        id: bevy_asset::UntypedAssetId,
    ) -> Option<&S> {
        self.get_resource::<bevy_asset::Assets<S>>()?
            .get(id.typed::<S>())
    }

    fn apply_source<R>(
        &mut self,
        id: bevy_asset::UntypedAssetId,
        f: impl FnOnce(&mut S) -> R,
    ) -> Option<R> {
        self.get_resource_mut::<bevy_asset::Assets<S>>()?
            .into_inner()
            .get_mut(id.typed::<S>())
            .map(f)
    }
}

pub type BevyTimeline = Timeline<World>;
pub type BevyTimelineBuilder<'a> = TimelineBuilder<'a, World>;
