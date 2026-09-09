pub mod bake;
mod func_pointers;
pub mod sample;

use core::any::TypeId;
use core::marker::PhantomData;
use core::time::Duration;

use bake::{BakeCtx, bake};
use func_pointers::{BakeFnPtr, SampleFnPtr};
use sample::{SampleCtx, sample};

use crate::ThreadSafe;
use crate::action::ActionKey;
use crate::pipeline::func_pointers::{BakeFn, SampleFn};
use crate::subject::SubjectId;
use crate::world::SubjectSource;

pub struct PipelineHandle<W, I, S, T> {
    #[expect(clippy::type_complexity)]
    _marker: PhantomData<fn() -> (W, I, S, T)>,
}

impl<W, I, S, T> PipelineHandle<W, I, S, T>
where
    W: 'static,
    I: SubjectId,
    S: 'static,
    T: 'static,
{
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    pub fn as_key(&self) -> PipelineKey {
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

    pub(crate) fn world_id(&self) -> TypeId {
        self.world_id
    }
}

/// A pipeline for baking and sampling actions of type `(I, S, T)`.
/// The world type `W` is erased at storage; it must match at call sites.
#[derive(Debug, Clone, Copy)]
pub struct Pipeline<W, I, S, T> {
    bake: BakeFn<W>,
    sample: SampleFn<W>,
    #[expect(clippy::type_complexity)]
    _marker: PhantomData<fn() -> (I, S, T)>,
}

impl<W, I, S, T> Pipeline<W, I, S, T> {
    pub fn new() -> Self
    where
        W: SubjectSource<I, S>,
        I: SubjectId,
        S: 'static,
        T: Clone + ThreadSafe,
    {
        Self {
            bake: bake::<W, I, S, T>,
            sample: sample::<W, I, S, T>,
            _marker: PhantomData,
        }
    }

    pub fn untyped(&self) -> PipelineUntyped {
        PipelineUntyped {
            bake: BakeFnPtr::new(self.bake),
            sample: SampleFnPtr::new(self.sample),
        }
    }
}

impl<W, I, S, T> Default for Pipeline<W, I, S, T>
where
    W: SubjectSource<I, S>,
    I: SubjectId,
    S: 'static,
    T: Clone + ThreadSafe,
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PipelineUntyped {
    bake: BakeFnPtr,
    sample: SampleFnPtr,
}

impl PipelineUntyped {
    /// # Safety
    ///
    /// `W` must match the type used when registering this pipeline.
    pub(crate) unsafe fn bake<W>(&self, ctx: BakeCtx<W>) {
        let f = unsafe { self.bake.typed_unchecked::<W>() };
        f(ctx)
    }

    /// # Safety
    ///
    /// `W` must match the type used when registering this pipeline.
    pub(crate) unsafe fn sample<W>(&self, ctx: SampleCtx<W>) {
        let f = unsafe { self.sample.typed_unchecked::<W>() };
        f(ctx)
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Range {
    pub start: Duration,
    pub end: Duration,
}

impl Range {
    /// Calculate if 2 [`Range`]s overlap.
    pub fn overlap(&self, other: &Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

#[cfg(test)]
mod tests {
    use crate::time::s;

    use super::*;

    #[test]
    fn range_overlap_behavior() {
        let a = Range {
            start: s(0),
            end: s(5),
        };
        let b = Range {
            start: s(3),
            end: s(8),
        };
        let c = Range {
            start: s(6),
            end: s(10),
        };
        let d = Range {
            start: s(5),
            end: s(5),
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
