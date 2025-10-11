use core::any::TypeId;

use bevy_ecs::prelude::*;
use bevy_platform::collections::HashMap;

use crate::accessor::{Accessor, FieldAccessorRegistry};
use crate::action::{
    ActionClip, ActionKey, ActionWorld, EaseStorage, InterpStorage,
    SampleMode, Segment,
};
use crate::field::UntypedField;
use crate::subject::SubjectId;
use crate::track::Track;
use crate::ThreadSafe;

/*
TODO: Convert Pipeline to be independant of Bevy's `World` as the
`target_world` for baking and sampling.

As such:
- `target_world` in Pipeline should be a trait/generic reference.
- `BakeCtx`/`SampleCtx` should only take in `target_world` with trait
  functions for getting the accessor or pipelines..?

See also https://github.com/voxell-tech/motiongfx/issues/71
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
    pub fn new(bake: BakeFn, sample: SampleFn) -> Self {
        Self { bake, sample }
    }

    pub fn bake(&self, ctx: BakeCtx) {
        (self.bake)(ctx)
    }

    pub fn sample(&self, ctx: SampleCtx) {
        (self.sample)(ctx)
    }
}

#[derive(Resource)]
pub struct PipelineRegistry {
    pipelines: HashMap<PipelineKey, Pipeline>,
}

impl PipelineRegistry {
    pub fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
        }
    }

    pub fn get(&self, key: &PipelineKey) -> Option<&Pipeline> {
        self.pipelines.get(key)
    }

    /// Register a pipeline function.
    ///
    /// Registering the same key twice will result in a replacement.
    ///
    /// # Note
    ///
    /// This function assumes that the baker function matches
    /// the field that it points towards. Failure to do so will
    /// result in a useless baker registry.
    pub fn register_unchecked(
        &mut self,
        key: PipelineKey,
        pipeline: Pipeline,
    ) -> &mut Self {
        self.pipelines.insert(key, pipeline);
        self
    }
}

impl Default for PipelineRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BakeCtx<'a> {
    pub track: &'a Track,
    pub action_world: &'a mut ActionWorld,
    pub target_world: &'a World,
    pub accessor_registry: &'a FieldAccessorRegistry,
}

impl<'a> BakeCtx<'a> {
    pub fn bake<I, S, T>(
        self,
        get_target: impl Fn(I, &'a World, Accessor<S, T>) -> Option<&'a T>,
    ) where
        I: SubjectId,
        S: 'static,
        T: Clone + ThreadSafe,
    {
        for (key, span) in self.track.sequences_spans() {
            let Ok(accessor) =
                self.accessor_registry.get::<S, T>(&key.field)
            else {
                continue;
            };

            let Some(&id) =
                self.action_world.get_id(&key.subject_id.uid)
            else {
                continue;
            };

            // Get the target value from the target world.
            let Some(mut start) =
                get_target(id, self.target_world, accessor).cloned()
            else {
                continue;
            };

            for ActionClip { id, .. } in self.track.clips(*span) {
                let Some(action) =
                    self.action_world.get_action::<T>(*id)
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

pub struct SampleCtx<'a> {
    pub action_world: &'a ActionWorld,
    pub target_world: &'a mut World,
    pub accessor_registry: &'a FieldAccessorRegistry,
}

impl<'a> SampleCtx<'a> {
    pub fn sample<I, S, T>(
        mut self,
        set_target: impl Fn(
            T,
            I,
            &'a mut World,
            Accessor<S, T>,
        ) -> &'a mut World,
    ) where
        I: SubjectId,
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

            let Some(&id) =
                self.action_world.get_id(&key.subject_id.uid)
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

            self.target_world =
                set_target(target, id, self.target_world, accessor);
        }
    }
}

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
    use crate::track::TrackBuilder;

    use super::*;

    #[test]
    fn new_pipeline() {
        let mut pipeline = PipelineRegistry::default();

        let _key0 = pipeline.register_component::<Transform, f32>();
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
