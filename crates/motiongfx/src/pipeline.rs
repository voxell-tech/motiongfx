use core::any::TypeId;

use bevy_ecs::prelude::*;
use bevy_platform::collections::HashMap;
use field_path::accessor::{Accessor, FieldAccessorRegistry};

use crate::ThreadSafe;
use crate::action::{
    ActionClip, ActionKey, ActionWorld, EaseStorage, InterpStorage,
    SampleMode, Segment,
};
use crate::subject::SubjectId;
use crate::track::Track;

/// Uniquely identifies a [`Pipeline`] to bake and sample a target
/// field from a subject's source data structure.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct PipelineKey {
    /// The [`TypeId`] of the [`SubjectId`].
    subject_id: TypeId,
    /// The [`TypeId`] of the source type.
    source_id: TypeId,
    /// The [`TypeId`] of the target type.
    target_id: TypeId,
}

impl PipelineKey {
    pub fn new<I, S, T>() -> Self
    where
        I: SubjectId,
        S: 'static,
        T: 'static,
    {
        Self {
            subject_id: TypeId::of::<I>(),
            source_id: TypeId::of::<S>(),
            target_id: TypeId::of::<T>(),
        }
    }

    pub fn from_action_key(key: ActionKey) -> Self {
        Self {
            subject_id: key.subject_id().type_id(),
            source_id: key.field().source_id(),
            target_id: key.field().target_id(),
        }
    }
}

pub type BakeFn<W> = fn(&W, BakeCtx);
pub type SampleFn<W> = fn(&mut W, SampleCtx);

#[derive(Debug, Clone, Copy)]
pub struct Pipeline<W> {
    bake: BakeFn<W>,
    sample: SampleFn<W>,
}

impl<W> Pipeline<W> {
    pub fn new(bake: BakeFn<W>, sample: SampleFn<W>) -> Self {
        Self { bake, sample }
    }

    pub fn bake(&self, world: &W, ctx: BakeCtx) {
        (self.bake)(world, ctx)
    }

    pub fn sample(&self, world: &mut W, ctx: SampleCtx) {
        (self.sample)(world, ctx)
    }
}

#[derive(Resource)]
pub struct PipelineRegistry<W> {
    pipelines: HashMap<PipelineKey, Pipeline<W>>,
}

impl<W> PipelineRegistry<W> {
    pub fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
        }
    }

    pub fn get(&self, key: &PipelineKey) -> Option<&Pipeline<W>> {
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
        pipeline: Pipeline<W>,
    ) -> &mut Self {
        self.pipelines.insert(key, pipeline);
        self
    }
}

impl<W> Default for PipelineRegistry<W> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BakeCtx<'a> {
    pub track: &'a Track,
    pub action_world: &'a mut ActionWorld,
    pub accessor_registry: &'a FieldAccessorRegistry,
}

impl<'a> BakeCtx<'a> {
    pub fn bake<I, S, T>(
        self,
        get_source: impl Fn(I) -> Option<&'a S>,
    ) where
        I: SubjectId,
        S: 'static,
        T: Clone + ThreadSafe,
    {
        for (key, span) in self.track.sequences_spans() {
            let Ok(accessor) =
                self.accessor_registry.get::<S, T>(key.field())
            else {
                continue;
            };

            let Some(&id) =
                self.action_world.get_id(&key.subject_id().uid())
            else {
                continue;
            };

            // Get the source from the target world.
            let Some(source) = get_source(id) else {
                continue;
            };

            let mut start = accessor.get_ref(source).clone();

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
    pub accessor_registry: &'a FieldAccessorRegistry,
}

impl<'a> SampleCtx<'a> {
    pub fn sample<I, S, T>(
        self,
        mut set_target: impl FnMut(I, T, Accessor<S, T>),
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
                self.accessor_registry.get::<S, T>(key.field())
            else {
                continue;
            };

            let Some(&id) =
                self.action_world.get_id(&key.subject_id().uid())
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

            set_target(id, target, accessor);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_overlap_behavior() {
        let a = Range {
            start: 0.0,
            end: 5.0,
        };
        let b = Range {
            start: 3.0,
            end: 8.0,
        };
        let c = Range {
            start: 6.0,
            end: 10.0,
        };
        let d = Range {
            start: 5.0,
            end: 5.0,
        }; // touching boundary

        assert!(
            a.overlap(&b),
            "Overlapping ranges should return true"
        );
        assert!(
            !a.overlap(&c),
            "Separated ranges should return false"
        );
        assert!(
            a.overlap(&d),
            "Touching at end should count as overlap"
        );
    }
}
