use alloc::vec::Vec;
use core::any::TypeId;
use core::marker::PhantomData;

use field_path::field::UntypedField;
use typarena::ColumnId;
use typarena::id::{GenId, IdGenerator};
use typarena::type_table::TypeTable;

use super::id_registry::{
    CleanupRegistry, IdRegistry, UId, cleanup_fn,
};
use super::{
    Action, ActionClip, ActionKey, ActionStorage, EaseFn,
    EaseStorage, InterpFn, InterpStorage, SampleMode, Segment,
    UntypedSubjectId,
};
use crate::ThreadSafe;
use crate::resources::Resources;
use crate::subject::SubjectId;
use crate::track::TrackFragment;

/// Phantom marker distinguishing [`ActionId`]'s [`GenId`] domain from
/// any other id domain.
pub struct ActionMarker;

/// The generational id backing [`ActionId`] and every column key in
/// an [`ActionTable`]'s [`TypeTable`].
pub type ActionId = GenId<ActionMarker>;

/// Heterogeneous storage for every action spawned via
/// [`Self::add`], keyed by [`ActionId`].
///
/// Each action's [`ActionKey`], [`ActionStorage`], and any optional
/// [`InterpStorage`]/[`EaseStorage`]/[`Segment`]/[`SampleMode`] live
/// in their own [`TypeTable`] column under the same id.
pub struct ActionTable {
    table: TypeTable<ActionId>,
    id_gen: IdGenerator<ActionMarker>,
    resources: Resources,
    // Cached `ColumnId`s for the non-generic columns touched on
    // every `add`/`remove`/mark, so those hot paths skip re-hashing
    // each `TypeId`.
    key_col: ColumnId,
    sample_mode_col: ColumnId,
    ease_col: ColumnId,
}

impl ActionTable {
    pub fn new() -> Self {
        let mut table = TypeTable::new();
        let key_col = table.ensure_column::<ActionKey>();
        let sample_mode_col = table.ensure_column::<SampleMode>();
        let ease_col = table.ensure_column::<EaseStorage>();

        Self {
            table,
            id_gen: IdGenerator::new(),
            resources: Resources::default(),
            key_col,
            sample_mode_col,
            ease_col,
        }
    }

    pub fn add<I, T>(
        &mut self,
        target: I,
        field: impl Into<UntypedField>,
        action: impl Action<T>,
    ) -> ActionBuilder<'_, T>
    where
        I: SubjectId,
        T: ThreadSafe,
    {
        let field = field.into();

        let uid = self
            .resources
            .get_or_insert_with(IdRegistry::new)
            .register_instance(target);
        self.resources
            .get_or_insert_with(CleanupRegistry::new)
            .insert(TypeId::of::<I>(), cleanup_fn::<I>);

        let key =
            ActionKey::new(UntypedSubjectId::new::<I>(uid), field);
        let id = self.id_gen.new_id();
        self.table.insert_by_column(id, key, self.key_col);
        self.table.insert(id, ActionStorage::new(action));

        ActionBuilder {
            table: &mut self.table,
            id,
            key,
            ease_col: self.ease_col,
            _phantom: PhantomData,
        }
    }

    pub fn remove(&mut self, id: ActionId) -> Option<ActionKey> {
        let key = *self
            .table
            .get_by_column::<ActionKey>(self.key_col, &id)?;

        let cleanup = self
            .resources
            .get::<CleanupRegistry>()
            .and_then(|fns| fns.get(&key.subject_id().type_id()))
            .copied();
        if let Some(cleanup) = cleanup {
            cleanup(&mut self.resources, key.subject_id().uid());
        }
        self.table.remove_row(&id);
        self.id_gen.recycle(id);

        Some(key)
    }

    pub fn get_action<T: ThreadSafe>(
        &self,
        id: &ActionId,
    ) -> Option<&impl Action<T>> {
        self.table.get::<ActionStorage<T>>(id).map(|a| &a.action)
    }

    pub fn get_id<I: SubjectId>(&self, uid: &UId) -> Option<&I> {
        self.resources.get::<IdRegistry<I>>()?.get_id(uid)
    }
}

impl ActionTable {
    /// Returns a immutable reference to the underlying storage.
    pub(crate) fn table(&self) -> &TypeTable<ActionId> {
        &self.table
    }

    /// Returns the [`ActionKey`] for `id`, using the cached column.
    pub(crate) fn key(&self, id: &ActionId) -> Option<&ActionKey> {
        self.table.get_by_column::<ActionKey>(self.key_col, id)
    }

    /// Returns the [`EaseStorage`] for `id`, if any, using the
    /// cached column.
    pub(crate) fn ease(&self, id: &ActionId) -> Option<&EaseStorage> {
        self.table.get_by_column::<EaseStorage>(self.ease_col, id)
    }

    /// Create an [`ActionCommand`] from an [`ActionId`].
    pub(crate) fn edit_action(
        &mut self,
        id: ActionId,
    ) -> ActionCommand<'_> {
        ActionCommand {
            table: &mut self.table,
            id,
            sample_mode_col: self.sample_mode_col,
        }
    }

    /// Remove [`SampleMode`] from all marked actions.
    pub(crate) fn clear_all_marks(&mut self) {
        let marked = self
            .table
            .iter::<SampleMode>()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();

        for id in marked {
            self.table.remove_by_column::<SampleMode>(
                &id,
                self.sample_mode_col,
            );
        }
    }
}

impl Default for ActionTable {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct ActionCommand<'w> {
    table: &'w mut TypeTable<ActionId>,
    id: ActionId,
    sample_mode_col: ColumnId,
}

impl ActionCommand<'_> {
    pub(crate) fn mark(
        &mut self,
        sample_mode: SampleMode,
    ) -> &mut Self {
        debug_assert!(self.table.contains_row(&self.id));
        self.table.insert_by_column(
            self.id,
            sample_mode,
            self.sample_mode_col,
        );
        self
    }

    pub(crate) fn clear_mark(&mut self) -> &mut Self {
        debug_assert!(self.table.contains_row(&self.id));
        self.table.remove_by_column::<SampleMode>(
            &self.id,
            self.sample_mode_col,
        );
        self
    }

    /// Add or replace the segment of the action.
    pub(crate) fn set_segment<T>(
        &mut self,
        segment: Segment<T>,
    ) -> &mut Self
    where
        T: ThreadSafe,
    {
        debug_assert!(self.table.contains_row(&self.id));
        self.table.insert(self.id, segment);
        self
    }
}

pub struct ActionBuilder<'w, T> {
    table: &'w mut TypeTable<ActionId>,
    id: ActionId,
    key: ActionKey,
    ease_col: ColumnId,
    _phantom: PhantomData<T>,
}

/// A builder struct to insert an interpolation method for the action
/// before compiling into an [`InterpActionBuilder`].
impl<T> ActionBuilder<'_, T> {
    /// Get the [`ActionId`] of the containing action.
    pub fn id(&self) -> ActionId {
        self.id
    }
}

impl<'w, T> ActionBuilder<'w, T>
where
    T: 'static,
{
    /// Set the interpolation method of the action.
    pub fn with_interp(
        self,
        interp: InterpFn<T>,
    ) -> InterpActionBuilder<'w, T> {
        self.table.insert(self.id, InterpStorage(interp));
        InterpActionBuilder { inner: self }
    }
}

/// An action builder that has interpolation added. This builder
/// exposes more customizations for the action and allows it to be
/// compiled into a [`TrackFragment`].
pub struct InterpActionBuilder<'w, T> {
    inner: ActionBuilder<'w, T>,
}

impl<T> InterpActionBuilder<'_, T> {
    /// Set the easing method of the action.
    pub fn with_ease(self, ease: EaseFn) -> Self {
        self.inner.table.insert_by_column(
            self.inner.id,
            EaseStorage(ease),
            self.inner.ease_col,
        );
        self
    }

    /// Get the [`ActionId`] of the containing action.
    pub fn id(&self) -> ActionId {
        self.inner.id()
    }

    /// Confirms the configuration of the action and creates a
    /// [`TrackFragment`].
    pub fn play(self, duration: f32) -> TrackFragment {
        TrackFragment::single(
            self.inner.key,
            ActionClip::new(self.id(), duration),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field() -> UntypedField {
        UntypedField::placeholder_with_path("x")
    }

    #[test]
    fn add_then_remove_round_trip() {
        let mut world = ActionTable::new();
        let id = world.add(1u32, field(), |x: &f32| x + 1.0).id();

        assert!(world.get_action::<f32>(&id).is_some());

        let key = world.remove(id);
        assert!(key.is_some());
        assert!(world.get_action::<f32>(&id).is_none());
    }

    #[test]
    fn id_recycling_produces_distinct_ids() {
        let mut world = ActionTable::new();

        let id1 = world.add(1u32, field(), |x: &f32| *x).id();
        world.remove(id1);

        let id2 = world.add(2u32, field(), |x: &f32| *x).id();

        // The recycled slot is reused, but the bumped generation
        // must keep the stale id from resolving to the new row.
        assert_ne!(id1, id2);
        assert!(world.get_action::<f32>(&id1).is_none());
        assert!(world.get_action::<f32>(&id2).is_some());
    }

    #[test]
    fn remove_decrements_registry_ref_count() {
        let mut world = ActionTable::new();

        // Two actions against the same `SubjectId` share one
        // `IdRegistry` entry (multi-instance ref-counting).
        let builder1 = world.add(7u32, field(), |x: &f32| *x);
        let uid = builder1.key.subject_id().uid();
        let id1 = builder1.id();

        let id2 = world.add(7u32, field(), |x: &f32| *x).id();

        assert_eq!(world.get_id::<u32>(&uid), Some(&7u32));

        world.remove(id1);
        // One instance remains, so the registry entry survives.
        assert_eq!(world.get_id::<u32>(&uid), Some(&7u32));

        world.remove(id2);
        // Last instance gone: the `Cleanup` closure must have
        // dropped the (now-empty) `IdRegistry<u32>` entry.
        assert_eq!(world.get_id::<u32>(&uid), None);
    }

    #[test]
    fn clear_all_marks_removes_all_sample_modes() {
        let mut world = ActionTable::new();

        let id1 = world.add(1u32, field(), |x: &f32| *x).id();
        let id2 = world.add(2u32, field(), |x: &f32| *x).id();

        world.edit_action(id1).mark(SampleMode::Start);
        world.edit_action(id2).mark(SampleMode::End);

        assert!(world.table().contains::<SampleMode>(&id1));
        assert!(world.table().contains::<SampleMode>(&id2));

        world.clear_all_marks();

        assert!(!world.table().contains::<SampleMode>(&id1));
        assert!(!world.table().contains::<SampleMode>(&id2));
    }
}
