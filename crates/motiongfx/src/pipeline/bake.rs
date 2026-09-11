use field_path::field::UntypedField;
use typarena::type_table::TypeTable;

use crate::ThreadSafe;
use crate::action::{
    ActionId, ActionTable, Segment, UntypedSubjectId,
};
use crate::registry::AccessorRegistry;
use crate::subject::SubjectId;
use crate::world::SubjectSource;

/// Working copies of the sources touched during one bake pass.
#[derive(Default)]
pub struct BakeScratch {
    sources: TypeTable<UntypedSubjectId>,
}

impl BakeScratch {
    /// The working copy of `subject`'s source, seeded from `seed` the
    /// first time it is asked for.
    fn source<S: Clone + ThreadSafe>(
        &mut self,
        subject: UntypedSubjectId,
        seed: impl FnOnce() -> Option<S>,
    ) -> Option<&mut S> {
        if !self.sources.contains::<S>(&subject) {
            self.sources.insert::<S>(subject, seed()?);
        }
        // TODO: collapse this contains + insert + get_mut once
        // typarena's `TypeTable` grows an entry API.
        Some(
            self.sources
                .get_mut::<S>(&subject)
                .expect("just inserted or already present"),
        )
    }
}

pub struct BakeClipCtx<'a, W> {
    pub world: &'a W,
    pub subject: UntypedSubjectId,
    pub field: UntypedField,
    pub action_id: ActionId,
    pub scratch: &'a mut BakeScratch,
    pub action_table: &'a mut ActionTable,
    pub accessor_registry: &'a AccessorRegistry,
}

/// Bakes one clip's [`Segment`] against the source's working copy, so
/// it composes on top of every earlier clip.
pub fn bake_clip<W, I, S, T>(ctx: BakeClipCtx<'_, W>)
where
    W: SubjectSource<I, S>,
    I: SubjectId,
    S: Clone + ThreadSafe,
    T: Clone + ThreadSafe,
{
    let Some(&subject_id) =
        ctx.action_table.get_id::<I>(&ctx.subject.uid())
    else {
        return;
    };
    let Some(accessor) =
        ctx.accessor_registry.get::<S, T>(&ctx.field)
    else {
        return;
    };
    let Some(source) = ctx.scratch.source::<S>(ctx.subject, || {
        ctx.world.get_source(subject_id).cloned()
    }) else {
        return;
    };
    let Some(action) =
        ctx.action_table.get_action::<T>(&ctx.action_id)
    else {
        return;
    };

    let start = accessor.get_ref(source).clone();
    let end = action(&start);

    let seg_col = ctx.action_table.ensure_segment_column::<T>();
    ctx.action_table.set_segment_by_column(
        ctx.action_id,
        Segment::new(start, end.clone()),
        seg_col,
    );

    *accessor.get_mut(source) = end;
}
