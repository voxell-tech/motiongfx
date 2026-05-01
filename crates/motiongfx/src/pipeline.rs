pub mod func_pointers;

use core::any::TypeId;
use core::marker::PhantomData;

use func_pointers::{BakeFnPtr, SampleFnPtr};

use crate::ThreadSafe;
use crate::action::{
    ActionClip, ActionKey, ActionWorld, EaseStorage, InterpStorage,
    SampleMode, Segment,
};
use crate::pipeline::func_pointers::{BakeFn, SampleFn};
use crate::registry::AccessorRegistry;
use crate::subject::SubjectId;
use crate::track::Track;

pub struct PipelineHandle<W, I, S, T> {
    #[expect(clippy::complexity)]
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
pub struct Pipeline<W, I, S, T> {
    bake: BakeFn<W>,
    sample: SampleFn<W>,
    #[expect(clippy::complexity)]
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
    use core::any::TypeId;

    use super::*;

    // ── Range ─────────────────────────────────────────────────────────────────

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

    #[test]
    fn range_overlap_is_symmetric() {
        let a = Range { start: 0.0, end: 3.0 };
        let b = Range { start: 2.0, end: 6.0 };
        assert_eq!(a.overlap(&b), b.overlap(&a));
    }

    #[test]
    fn range_no_overlap_adjacent_ranges() {
        // [0, 1) and (1, 2] do not overlap, but touching is fine.
        let a = Range { start: 0.0, end: 1.0 };
        let b = Range { start: 1.0, end: 2.0 };
        // Our implementation treats touching as overlap (<=).
        assert!(a.overlap(&b));
    }

    // ── PipelineKey ───────────────────────────────────────────────────────────

    /// Minimal subject ID type used in tests.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct MockId(u32);

    /// Dummy source type.
    #[derive(Clone)]
    struct MockSource {
        pub value: f32,
    }

    /// First world type.
    struct WorldA;
    /// Second world type (different from WorldA).
    struct WorldB;

    impl SubjectSource<MockId, MockSource> for WorldA {
        fn get_source(&self, _id: MockId) -> Option<&MockSource> { None }
        fn apply_source<R>(&mut self, _id: MockId, _f: impl FnOnce(&mut MockSource) -> R) -> Option<R> { None }
    }

    impl SubjectSource<MockId, MockSource> for WorldB {
        fn get_source(&self, _id: MockId) -> Option<&MockSource> { None }
        fn apply_source<R>(&mut self, _id: MockId, _f: impl FnOnce(&mut MockSource) -> R) -> Option<R> { None }
    }

    #[test]
    fn pipeline_key_new_includes_world_type_id() {
        let key = PipelineKey::new::<WorldA, MockId, MockSource, f32>();
        assert_eq!(key.world_id(), TypeId::of::<WorldA>());
    }

    #[test]
    fn pipeline_key_same_types_produce_equal_keys() {
        let key1 = PipelineKey::new::<WorldA, MockId, MockSource, f32>();
        let key2 = PipelineKey::new::<WorldA, MockId, MockSource, f32>();
        assert_eq!(key1, key2);
    }

    #[test]
    fn pipeline_key_different_world_types_produce_different_keys() {
        let key_a = PipelineKey::new::<WorldA, MockId, MockSource, f32>();
        let key_b = PipelineKey::new::<WorldB, MockId, MockSource, f32>();
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn pipeline_key_different_target_types_produce_different_keys() {
        let key_f32 = PipelineKey::new::<WorldA, MockId, MockSource, f32>();
        let key_u32 = PipelineKey::new::<WorldA, MockId, MockSource, u32>();
        assert_ne!(key_f32, key_u32);
    }

    // ── PipelineHandle ────────────────────────────────────────────────────────

    #[test]
    fn pipeline_handle_as_key_matches_pipeline_key_new() {
        let handle = PipelineHandle::<WorldA, MockId, MockSource, f32>::new();
        let key_from_handle = handle.as_key();
        let key_direct = PipelineKey::new::<WorldA, MockId, MockSource, f32>();
        assert_eq!(key_from_handle, key_direct);
    }

    #[test]
    fn pipeline_handle_is_copy_and_clone() {
        let handle = PipelineHandle::<WorldA, MockId, MockSource, f32>::new();
        let _copy = handle;
        let _cloned = handle.clone();
    }

    // ── SubjectSource ─────────────────────────────────────────────────────────

    /// A simple world backed by a Vec for unit tests.
    struct VecWorld(alloc::vec::Vec<f32>);

    impl SubjectSource<usize, f32> for VecWorld {
        fn get_source(&self, id: usize) -> Option<&f32> {
            self.0.get(id)
        }

        fn apply_source<R>(
            &mut self,
            id: usize,
            f: impl FnOnce(&mut f32) -> R,
        ) -> Option<R> {
            self.0.get_mut(id).map(f)
        }
    }

    #[test]
    fn subject_source_get_returns_value() {
        let world = VecWorld(alloc::vec![1.0, 2.0, 3.0]);
        assert_eq!(world.get_source(0), Some(&1.0));
        assert_eq!(world.get_source(1), Some(&2.0));
        assert_eq!(world.get_source(99), None);
    }

    #[test]
    fn subject_source_apply_mutates_value() {
        let mut world = VecWorld(alloc::vec![0.0]);
        world.apply_source(0, |v| *v = 5.0);
        assert_eq!(world.0[0], 5.0);
    }

    #[test]
    fn subject_source_apply_returns_none_for_missing_id() {
        let mut world = VecWorld(alloc::vec![0.0]);
        let result = world.apply_source(99, |v| *v = 1.0);
        assert!(result.is_none());
    }

    // ── Pipeline ──────────────────────────────────────────────────────────────

    #[test]
    fn pipeline_new_produces_untyped_pipeline() {
        let pipeline =
            Pipeline::<VecWorld, usize, f32, f32>::new();
        // Calling `untyped()` should succeed without panicking.
        let _untyped = pipeline.untyped();
    }
}
