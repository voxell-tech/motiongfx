use core::any::TypeId;

#[cfg(feature = "asset")]
use bevy_asset::{AsAssetId, Assets};
use bevy_ecs::component::Mutable;
use bevy_ecs::prelude::*;
use bevy_platform::collections::HashMap;

use crate::accessor::{Accessor, FieldAccessorRegistry};
use crate::action::{
    ActionClip, ActionWorld, EaseStorage, InterpStorage, SampleMode,
    Segment,
};
use crate::field::UntypedField;
use crate::track::{ActionKey, Track};
use crate::ThreadSafe;

/*
GOAL: Convert Pipeline to be independant of Bevy's `World` as the
`target_world` for baking and sampling.

As such:
- `target_world` in Pipeline should be a trait/generic reference.
- `TargetAction` should be a generic in the entire ecosystem.
*/

pub type BakeFn = fn(BakeCtx);
pub type SampleFn = fn(SampleCtx);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct PipelineKey {
    /// The [`TypeId`] of the source type.
    source_id: TypeId,
    /// The [`TypeId`] of the target type.
    target_id: TypeId,
}

impl PipelineKey {
    pub fn new<S: 'static, T: 'static>() -> Self {
        Self {
            source_id: TypeId::of::<S>(),
            target_id: TypeId::of::<T>(),
        }
    }

    pub fn from_field(field: impl Into<UntypedField>) -> Self {
        let field = field.into();
        Self {
            source_id: field.source_id(),
            target_id: field.target_id(),
        }
    }
}

impl From<UntypedField> for PipelineKey {
    fn from(field: UntypedField) -> Self {
        Self::from_field(field)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Pipeline {
    bake: BakeFn,
    sample: SampleFn,
}

impl Pipeline {
    pub fn new_component<S, T>() -> Self
    where
        S: Component<Mutability = Mutable>,
        T: Clone + ThreadSafe,
    {
        Self {
            bake: bake_component_actions::<S, T>,
            sample: sample_component_actions::<S, T>,
        }
    }

    #[cfg(feature = "asset")]
    pub fn new_asset<S, T>() -> Self
    where
        S: AsAssetId,
        T: Clone + ThreadSafe,
    {
        Self {
            bake: bake_asset_actions::<S, T>,
            sample: sample_asset_actions::<S, T>,
        }
    }

    pub fn bake(&self, ctx: BakeCtx) {
        (self.bake)(ctx)
    }

    pub fn sample(&self, ctx: SampleCtx) {
        (self.sample)(ctx)
    }
}

#[derive(Resource, Default)]
pub struct PipelineRegistry {
    pipelines: HashMap<PipelineKey, Pipeline>,
}

impl PipelineRegistry {
    /// Registers a pipeline for a given component and the
    /// target field.
    ///
    /// Will overwrite existing accessor.
    pub fn register_component<S, T>(&mut self) -> PipelineKey
    where
        S: Component<Mutability = Mutable>,
        T: Clone + ThreadSafe,
    {
        let key = PipelineKey::new::<S, T>();

        // Prevent registering the same key twice.
        if self.pipelines.contains_key(&key) {
            return key;
        }

        unsafe {
            self.register_unchecked(
                key,
                Pipeline::new_component::<S, T>(),
            );
        }

        key
    }

    /// Registers a pipeline for a given asset and the
    /// target field.
    ///
    /// Will overwrite existing accessor.
    #[cfg(feature = "asset")]
    pub fn register_asset<S, T>(&mut self) -> PipelineKey
    where
        S: AsAssetId,
        T: Clone + ThreadSafe,
    {
        let key = PipelineKey::new::<S, T>();

        // Prevent registering the same key twice.
        if self.pipelines.contains_key(&key) {
            return key;
        }

        unsafe {
            self.register_unchecked(
                key,
                Pipeline::new_asset::<S, T>(),
            );
        }

        key
    }
}

impl PipelineRegistry {
    pub fn get(&self, key: &PipelineKey) -> Option<&Pipeline> {
        self.pipelines.get(key)
    }

    /// Register a pipeline function.
    ///
    /// Registering the same key twice will result in a replacement.
    ///
    /// # Safety
    ///
    /// This function assumes that the baker function matches
    /// the field that it points towards. Failure to do so will
    /// result in a useless baker registry.
    pub unsafe fn register_unchecked(
        &mut self,
        key: PipelineKey,
        pipeline: Pipeline,
    ) -> &mut Self {
        self.pipelines.insert(key, pipeline);
        self
    }
}

pub struct BakeCtx<'a> {
    pub track: &'a Track,
    pub action_world: &'a mut ActionWorld,
    pub target_world: &'a World,
    pub accessor_registry: &'a FieldAccessorRegistry,
}

impl<'a> BakeCtx<'a> {
    pub fn bake<S, T>(
        self,
        get_target: impl Fn(
            Entity,
            &'a World,
            Accessor<S, T>,
        ) -> Option<&'a T>,
    ) where
        S: 'static,
        T: Clone + ThreadSafe,
    {
        for (key, span) in self.track.sequences_spans() {
            let Ok(accessor) =
                self.accessor_registry.get::<S, T>(&key.field)
            else {
                continue;
            };

            let mut target_entity = key.target.0;

            // Fetch target reference if any.
            target_entity = self
                .target_world
                .get::<TargetRef>(target_entity)
                .map(|r| r.0)
                .unwrap_or(target_entity);

            // Get the target value from the target world.
            let Some(mut start) = get_target(
                target_entity,
                self.target_world,
                accessor,
            )
            .cloned() else {
                continue;
            };

            for ActionClip { id, .. } in self.track.clips(*span) {
                let Some(action) = self.action_world.get::<T>(*id)
                else {
                    continue;
                };

                let end = action(&start);
                let segment =
                    Segment::new(start.clone(), end.clone());

                self.action_world
                    .edit_action(*id)
                    .set_segment(segment);

                start = end;
            }
        }
    }
}

pub fn bake_component_actions<S, T>(ctx: BakeCtx)
where
    S: Component,
    T: Clone + ThreadSafe,
{
    ctx.bake::<S, T>(|target_entity, target_world, accessor| {
        target_world
            .get::<S>(target_entity)
            .map(|s| (accessor.ref_fn)(s))
    });
}

#[cfg(feature = "asset")]
pub fn bake_asset_actions<S, T>(ctx: BakeCtx)
where
    S: AsAssetId,
    T: Clone + ThreadSafe,
{
    let Some(assets) =
        ctx.target_world.get_resource::<Assets<S::Asset>>()
    else {
        return;
    };

    ctx.bake::<S::Asset, T>(
        |target_entity, target_world, accessor| {
            target_world
                .get::<S>(target_entity)
                .and_then(|s| assets.get(s.as_asset_id()))
                .map(|s| (accessor.ref_fn)(s))
        },
    );
}

pub struct SampleCtx<'a> {
    pub action_world: &'a ActionWorld,
    pub target_world: &'a mut World,
    pub accessor_registry: &'a FieldAccessorRegistry,
}

impl<'a> SampleCtx<'a> {
    pub fn sample<S, T>(
        mut self,
        set_target: impl Fn(
            T,
            Entity,
            &'a mut World,
            Accessor<S, T>,
        ) -> &'a mut World,
    ) where
        S: 'static,
        T: Clone + ThreadSafe,
    {
        let Some(mut q) = self.action_world.world().try_query::<(
            &ActionKey,
            &SampleMode,
            &Segment<T>,
            &InterpStorage<T>,
            Option<&EaseStorage>,
        )>() else {
            return;
        };

        for (key, sample_mode, segment, interp, ease) in
            q.iter(self.action_world.world())
        {
            let Ok(accessor) =
                self.accessor_registry.get::<S, T>(&key.field)
            else {
                continue;
            };

            let target = match sample_mode {
                SampleMode::Start => segment.start.clone(),
                SampleMode::End => segment.end.clone(),
                SampleMode::Interp(t) => {
                    let t = match ease {
                        Some(ease) => ease.0(*t),
                        None => *t,
                    };

                    interp.0(&segment.start, &segment.end, t)
                }
            };

            let mut target_entity = key.target.0;

            // Fetch target reference if any.
            target_entity = self
                .target_world
                .get::<TargetRef>(target_entity)
                .map(|r| r.0)
                .unwrap_or(target_entity);

            self.target_world = set_target(
                target,
                target_entity,
                self.target_world,
                accessor,
            );
        }
    }
}

pub fn sample_component_actions<S, T>(ctx: SampleCtx)
where
    S: Component<Mutability = Mutable>,
    T: Clone + ThreadSafe,
{
    ctx.sample::<S, T>(
        |target, target_entity, target_world, accessor| {
            if let Some(mut source) =
                target_world.get_mut::<S>(target_entity)
            {
                *(accessor.mut_fn)(&mut source) = target;
            }

            target_world
        },
    );
}

#[cfg(feature = "asset")]
pub fn sample_asset_actions<S, T>(ctx: SampleCtx)
where
    S: AsAssetId,
    T: Clone + ThreadSafe,
{
    ctx.sample::<S::Asset, T>(
        |target, target_entity, target_world, accessor| {
            // Get asset id.
            let Some(id) = target_world
                .get::<S>(target_entity)
                .map(|c| c.as_asset_id())
            else {
                return target_world;
            };

            // Get assets resource.
            let Some(mut assets) =
                target_world.get_resource_mut::<Assets<S::Asset>>()
            else {
                return target_world;
            };

            // Writes target value.
            if let Some(source) = assets.get_mut(id) {
                *(accessor.mut_fn)(source) = target;
            }

            target_world
        },
    );
}

// TODO: Should we support recursive re-direction?

/// A re-direction from an entity to another when dealing with
/// action baking/sampling. The user is responsible for the
/// existance of the referenced entity.
#[derive(Component)]
pub struct TargetRef(pub Entity);

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub struct Range {
    pub start: f32,
    pub end: f32,
}

impl Range {
    /// Calculate if 2 [`Range`]s overlap.
    pub fn overlap(&self, other: &Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

/*
#[cfg(test)]
mod tests {
    use crate::timeline_v3::track::TrackBuilder;

    use super::*;

    #[test]
    fn new_pipeline() {
        let mut pipeline = PipelineRegistry::default();

        let _key0 = pipeline.register_comp::<Transform, f32>();
        let _key1 = pipeline
            .register_asset::<MeshMaterial3d<StandardMaterial>, f32>(
            );

        let transform_pipeline = pipeline.get(&_key0).unwrap();

        let mut action_world = ActionWorld::new();
        let mut target_world = World::new();
        let field_registry = FieldRegistry::new();
        let track = TrackBuilder::default().compile();

        transform_pipeline.bake(BakeCtx {
            action_world: &mut action_world,
            target_world: &mut target_world,
            field_registry: &field_registry,
            track: &track,
        });

        transform_pipeline.sample(SampleCtx {
            action_world: &action_world,
            target_world: &mut target_world,
            field_registry: &field_registry,
        });

        let mut world = World::new();
        world.insert_resource(pipeline);
    }
}
*/
