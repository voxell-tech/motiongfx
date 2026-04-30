pub mod func_pointers;

use core::any::TypeId;
use core::marker::PhantomData;

use func_pointers::{BakeFnPtr, SampleFnPtr};

use crate::ThreadSafe;
use crate::action::{
    ActionClip, ActionKey, ActionWorld, EaseStorage, InterpStorage,
    SampleMode, Segment,
};
use crate::registry::AccessorRegistry;
use crate::subject::SubjectId;
use crate::track::Track;

pub struct PipelineHandle<W, I, S, T> {
    #[expect(clippy::complexity)]
    _marker: PhantomData<fn() -> (W, I, S, T)>,
}

impl<W, I, S, T> PipelineHandle<W, I, S, T> {
    pub fn new() -> Self
    where
        W: 'static,
        I: SubjectId,
        S: 'static,
        T: 'static,
    {
        Self {
            _marker: PhantomData,
        }
    }

    pub fn as_key(&self) -> PipelineKey
    where
        W: 'static,
        I: SubjectId,
        S: 'static,
        T: 'static,
    {
        PipelineKey::new::<W, I, S, T>()
    }
}

impl<W, I, S, T> Copy for PipelineHandle<W, I, S, T> {}

impl<W, I, S, T> Clone for PipelineHandle<W, I, S, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<W, I, S, T> Default for PipelineHandle<W, I, S, T>
where
    W: 'static,
    I: SubjectId,
    S: 'static,
    T: 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Uniquely identifies a [`Pipeline`] by its world, subject, source,
/// and target types.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct PipelineKey {
    world_id: TypeId,
    subject_id: TypeId,
    source_id: TypeId,
    target_id: TypeId,
}

impl PipelineKey {
    pub fn new<W, I, S, T>() -> Self
    where
        W: 'static,
        I: SubjectId,
        S: 'static,
        T: 'static,
    {
        Self {
            world_id: TypeId::of::<W>(),
            subject_id: TypeId::of::<I>(),
            source_id: TypeId::of::<S>(),
            target_id: TypeId::of::<T>(),
        }
    }

    pub fn from_action_key<W: 'static>(key: ActionKey) -> Self {
        Self {
            world_id: TypeId::of::<W>(),
            subject_id: key.subject_id().type_id(),
            source_id: key.field().source_id(),
            target_id: key.field().target_id(),
        }
    }
}

/// Provides read and write access to a source type `S` by subject id `I`.
pub trait SubjectSource<I: SubjectId, S: 'static> {
    fn get_source(&self, id: I) -> Option<&S>;
    fn apply_source<R>(
        &mut self,
        id: I,
        f: impl FnOnce(&mut S) -> R,
    ) -> Option<R>;
}

/// A pipeline for baking and sampling actions of type `(I, S, T)`.
/// The world type `W` is erased at storage; it must match at call sites.
#[derive(Debug, Clone, Copy)]
pub struct Pipeline<I, S, T> {
    bake: BakeFnPtr,
    sample: SampleFnPtr,
    #[expect(clippy::complexity)]
    _marker: PhantomData<fn() -> (I, S, T)>,
}

impl<I, S, T> Pipeline<I, S, T> {
    pub fn new<W>() -> Self
    where
        W: SubjectSource<I, S>,
        I: SubjectId,
        S: 'static,
        T: Clone + ThreadSafe,
    {
        Self {
            bake: BakeFnPtr::new(bake::<W, I, S, T>),
            sample: SampleFnPtr::new(sample::<W, I, S, T>),
            _marker: PhantomData,
        }
    }

    pub fn untyped(&self) -> PipelineUntyped {
        PipelineUntyped {
            bake: self.bake,
            sample: self.sample,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PipelineUntyped {
    bake: BakeFnPtr,
    sample: SampleFnPtr,
}

impl PipelineUntyped {
    pub fn bake<W>(&self, ctx: BakeCtx<W>) {
        // SAFETY: W matches the W passed to Pipeline::new.
        let f = unsafe { self.bake.typed_unchecked::<W>() };
        f(ctx)
    }

    pub fn sample<W>(&self, ctx: SampleCtx<W>) {
        // SAFETY: W matches the W passed to Pipeline::new.
        let f = unsafe { self.sample.typed_unchecked::<W>() };
        f(ctx)
    }
}

pub struct BakeCtx<'a, W> {
    pub world: &'a W,
    pub track: &'a Track,
    pub action_world: &'a mut ActionWorld,
    pub accessor_registry: &'a AccessorRegistry,
}

pub fn bake<W, I, S, T>(ctx: BakeCtx<W>)
where
    W: SubjectSource<I, S>,
    I: SubjectId,
    S: 'static,
    T: Clone + ThreadSafe,
{
    for (key, span) in ctx.track.sequences_spans() {
        let Some(accessor) =
            ctx.accessor_registry.get::<S, T>(key.field())
        else {
            continue;
        };

        let Some(&id) =
            ctx.action_world.get_id(&key.subject_id().uid())
        else {
            continue;
        };

        let Some(source) = ctx.world.get_source(id) else {
            continue;
        };

        let mut start = accessor.get_ref(source).clone();

        for ActionClip { id, .. } in ctx.track.clips(*span) {
            let Some(action) = ctx.action_world.get_action::<T>(*id)
            else {
                continue;
            };

            let end = action(&start);
            let segment = Segment::new(start.clone(), end.clone());

            ctx.action_world.edit_action(*id).set_segment(segment);

            start = end;
        }
    }
}

pub struct SampleCtx<'a, W> {
    pub world: &'a mut W,
    pub action_world: &'a ActionWorld,
    pub accessor_registry: &'a AccessorRegistry,
}

pub fn sample<W, I, S, T>(ctx: SampleCtx<W>)
where
    W: SubjectSource<I, S>,
    I: SubjectId,
    S: 'static,
    T: Clone + ThreadSafe,
{
    let Some(mut q) = ctx.action_world.world().try_query::<(
        &ActionKey,
        &SampleMode,
        &Segment<T>,
        &InterpStorage<T>,
        Option<&EaseStorage>,
    )>() else {
        return;
    };

    for (key, sample_mode, segment, interp, ease) in
        q.iter(ctx.action_world.world())
    {
        let Some(accessor) =
            ctx.accessor_registry.get::<S, T>(key.field())
        else {
            continue;
        };

        let Some(&id) =
            ctx.action_world.get_id(&key.subject_id().uid())
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

        ctx.world.apply_source(id, |source| {
            *accessor.get_mut(source) = target;
        });
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
