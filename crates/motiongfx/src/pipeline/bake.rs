use crate::ThreadSafe;
use crate::action::{ActionClip, ActionTable, Segment};
use crate::registry::AccessorRegistry;
use crate::subject::SubjectId;
use crate::track::Track;
use crate::world::SubjectSource;

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
    let segment_col = ctx.action_table.ensure_segment_column::<T>();

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

        let mut start = accessor.get_ref(source).clone();

        for ActionClip { id, .. } in ctx.track.clips(*span) {
            let Some(action) = ctx
                .action_table
                .get_action_by_column::<T>(action_col, id)
            else {
                continue;
            };

            let end = action(&start);
            let segment = Segment::new(start.clone(), end.clone());

            ctx.action_table.set_segment_by_column(
                *id,
                segment,
                segment_col,
            );

            start = end;
        }
    }
}
