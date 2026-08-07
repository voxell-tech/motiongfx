pub mod func_pointers;

use core::any::TypeId;
use core::marker::PhantomData;
use core::time::Duration;

use alloc::vec::Vec;

use func_pointers::{BakeFnPtr, SampleFnPtr};

use crate::ThreadSafe;
use crate::action::{
    ActionClip, ActionId, ActionKey, ActionTable, InterpStorage,
    SampleMode, Segment,
};
use crate::pipeline::func_pointers::{BakeFn, SampleFn};
use crate::registry::AccessorRegistry;
use crate::subject::SubjectId;
use crate::timeline::resolve_clip;
use crate::track::Track;
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

pub struct BakeCtx<'a, W> {
    pub world: &'a W,
    pub track: &'a Track,
    pub action_table: &'a mut ActionTable,
    pub accessor_registry: &'a AccessorRegistry,
}

pub fn bake<W, I, S, T>(ctx: BakeCtx<W>)
where
    W: SubjectSource<I, S>,
    I: SubjectId,
    S: 'static,
    T: Clone + ThreadSafe,
{
    // Resolve the per-`T` columns once so the clip loop doesn't
    // re-hash the `TypeId` on every access. No `T` action, no bake.
    let Some(action_col) = ctx.action_table.action_column::<T>()
    else {
        return;
    };
    let interp_col =
        ctx.action_table.table().type_column::<InterpStorage<T>>();
    let segment_col = ctx.action_table.ensure_segment_column::<T>();

    // The lane baked so far, index for index. A skipped clip is in
    // neither, so `resolve_clip` only picks one with a value to read.
    let mut baked = Vec::<ActionClip>::new();
    let mut segments = Vec::<Segment<T>>::new();

    for (key, span) in ctx.track.sequences_spans() {
        let Some(accessor) =
            ctx.accessor_registry.get::<S, T>(key.field())
        else {
            continue;
        };

        let Some(&id) =
            ctx.action_table.get_id(&key.subject_id().uid())
        else {
            continue;
        };

        let Some(source) = ctx.world.get_source(id) else {
            continue;
        };

        // A running argmax of clip ends, so a lane that never
        // overlaps skips `resolve_clip`.
        let mut max_end = Duration::ZERO;
        let mut max_idx = 0;

        for clip in ctx.track.clips(*span) {
            let Some(action) = ctx
                .action_table
                .get_action_by_column::<T>(action_col, &clip.id)
            else {
                continue;
            };

            // Open on what the lane is showing here, not on whatever
            // ran last.
            let start = if baked.is_empty() {
                // Nothing has run yet, so the subject is untouched.
                accessor.get_ref(source).clone()
            } else {
                // Nothing is still running, so nothing covers this
                // clip.
                let (i, mode) = if max_end < clip.start {
                    (max_idx, SampleMode::End)
                } else {
                    resolve_clip(&baked, clip.start)
                };

                let segment = &segments[i];
                let under = baked[i].id;

                match mode {
                    // Unreachable on a start-sorted lane, and the
                    // right value if that ever broke.
                    SampleMode::Start => {
                        debug_assert!(
                            false,
                            "lane is not sorted by start"
                        );
                        segment.start.clone()
                    }
                    SampleMode::End => segment.end.clone(),
                    SampleMode::Interp(t) => {
                        let interp = interp_col.and_then(|col| {
                            ctx.action_table
                                .table()
                                .get_by_column::<InterpStorage<T>>(
                                    col, &under,
                                )
                        });

                        match interp {
                            Some(interp) => {
                                let t = match ctx
                                    .action_table
                                    .ease(&under)
                                {
                                    Some(ease) => ease.0(t),
                                    None => t,
                                };

                                interp.0(
                                    &segment.start,
                                    &segment.end,
                                    t,
                                )
                            }
                            // TODO: an action with no interpolation
                            // is never drawn, so this opens the next
                            // clip on a value the lane never showed.
                            None => segment.end.clone(),
                        }
                    }
                }
            };

            let end = action(&start);

            baked.push(*clip);
            segments.push(Segment::new(start, end));

            if clip.end() >= max_end {
                max_end = clip.end();
                max_idx = baked.len() - 1;
            }
        }

        for (clip, segment) in baked.drain(..).zip(segments.drain(..))
        {
            ctx.action_table.set_segment_by_column(
                clip.id,
                segment,
                segment_col,
            );
        }
    }
}

pub struct SampleCtx<'a, W> {
    pub world: &'a mut W,
    pub action_table: &'a ActionTable,
    pub accessor_registry: &'a AccessorRegistry,
    /// The queued actions for this pipeline, each with its
    /// [`SampleMode`] resolved at queue time.
    pub samples: &'a [(ActionId, SampleMode)],
}

pub fn sample<W, I, S, T>(ctx: SampleCtx<W>)
where
    W: SubjectSource<I, S>,
    I: SubjectId,
    S: 'static,
    T: Clone + ThreadSafe,
{
    let table = ctx.action_table.table();
    let Some(segment_col) = table.type_column::<Segment<T>>() else {
        return;
    };
    let Some(interp_col) = table.type_column::<InterpStorage<T>>()
    else {
        return;
    };

    for &(id, sample_mode) in ctx.samples {
        let Some(segment) =
            table.get_by_column::<Segment<T>>(segment_col, &id)
        else {
            continue;
        };
        let Some(interp) =
            table.get_by_column::<InterpStorage<T>>(interp_col, &id)
        else {
            continue;
        };

        let Some(key) = ctx.action_table.key(&id) else {
            continue;
        };
        let ease = ctx.action_table.ease(&id);
        let Some(accessor) =
            ctx.accessor_registry.get::<S, T>(key.field())
        else {
            continue;
        };

        let Some(&sid) =
            ctx.action_table.get_id(&key.subject_id().uid())
        else {
            continue;
        };

        let target = match sample_mode {
            SampleMode::Start => segment.start.clone(),
            SampleMode::End => segment.end.clone(),
            SampleMode::Interp(t) => {
                let t = match ease {
                    Some(ease) => ease.0(t),
                    None => t,
                };

                interp.0(&segment.start, &segment.end, t)
            }
        };

        ctx.world.apply_source(sid, |source| {
            *accessor.get_mut(source) = target;
        });
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
    use crate::interpolation::Interpolation;
    use crate::time::s;

    use super::*;

    struct MockWorld(f32);

    impl SubjectSource<u32, f32> for MockWorld {
        fn get_source(&self, _id: u32) -> Option<&f32> {
            Some(&self.0)
        }

        fn apply_source<R>(
            &mut self,
            _id: u32,
            f: impl FnOnce(&mut f32) -> R,
        ) -> Option<R> {
            Some(f(&mut self.0))
        }
    }

    fn sample_mock(
        action_table: &ActionTable,
        accessor_registry: &AccessorRegistry,
        world: &mut MockWorld,
        samples: &[(ActionId, SampleMode)],
    ) {
        sample::<MockWorld, u32, f32, f32>(SampleCtx {
            world,
            action_table,
            accessor_registry,
            samples,
        });
    }

    /// Exercises the multi-column probe in `sample`: `ActionKey`,
    /// `Segment<T>` and `InterpStorage<T>` are read per queued action,
    /// with the `SampleMode` supplied by the queue.
    #[test]
    fn sample_join_reads_all_required_columns() {
        let field_acc = crate::path!(<f32>);
        let field = field_acc.field.untyped();

        let mut accessor_registry = AccessorRegistry::new();
        accessor_registry.register(field_acc);

        let mut action_table = ActionTable::new();
        let id = action_table
            .add(0u32, field, |x: &f32| *x + 10.0)
            .with_interp(<f32 as Interpolation<()>>::interp)
            .id();
        let seg_col = action_table.ensure_segment_column::<f32>();
        action_table.set_segment_by_column(
            id,
            Segment::new(0.0f32, 10.0f32),
            seg_col,
        );

        let mut world = MockWorld(0.0);

        sample_mock(
            &action_table,
            &accessor_registry,
            &mut world,
            &[(id, SampleMode::Start)],
        );
        assert_eq!(world.0, 0.0);

        sample_mock(
            &action_table,
            &accessor_registry,
            &mut world,
            &[(id, SampleMode::End)],
        );
        assert_eq!(world.0, 10.0);

        sample_mock(
            &action_table,
            &accessor_registry,
            &mut world,
            &[(id, SampleMode::Interp(0.5))],
        );
        assert!((world.0 - 5.0).abs() < f32::EPSILON);
    }

    /// The optional `EaseStorage` column must be probed too: a
    /// present column should reshape `t` before interpolating.
    #[test]
    fn sample_join_applies_custom_ease() {
        let field_acc = crate::path!(<f32>);
        let field = field_acc.field.untyped();

        let mut accessor_registry = AccessorRegistry::new();
        accessor_registry.register(field_acc);

        let mut action_table = ActionTable::new();
        let id = action_table
            .add(0u32, field, |x: &f32| *x + 10.0)
            .with_interp(<f32 as Interpolation<()>>::interp)
            .with_ease(crate::ease::quad::ease_in)
            .id();
        let seg_col = action_table.ensure_segment_column::<f32>();
        action_table.set_segment_by_column(
            id,
            Segment::new(0.0f32, 10.0f32),
            seg_col,
        );
        let mut world = MockWorld(0.0);
        sample_mock(
            &action_table,
            &accessor_registry,
            &mut world,
            &[(id, SampleMode::Interp(0.5))],
        );

        // quad::ease_in(0.5) == 0.25, so the eased target is 2.5,
        // not the unmodified-t value of 5.0.
        assert!((world.0 - 2.5).abs() < f32::EPSILON);
    }

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
