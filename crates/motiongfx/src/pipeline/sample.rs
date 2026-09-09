use crate::ThreadSafe;
use crate::action::{
    ActionId, ActionTable, InterpStorage, SampleMode, Segment,
};
use crate::registry::AccessorRegistry;
use crate::subject::SubjectId;
use crate::world::SubjectSource;

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

#[cfg(test)]
mod tests {
    use motiongfx_interp::interpolation::Interpolation;

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
            .with_ease(motiongfx_interp::ease::quad::ease_in)
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
}
