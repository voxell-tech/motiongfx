//! Per-backend value storage.
//!
//! `ActionCmd`/`FieldSeed` reference a [`SceneBackend::ValueId`](crate::backend::SceneBackend::ValueId),
//! resolved through the backend's own
//! [`SceneBackend::ValuePool`](crate::backend::SceneBackend::ValuePool)
//! via [`ValueColumn`] - one small impl per concrete value type. No
//! wrapper type around the value, no enum: a backend's pool is a plain
//! struct of named columns (one per value type it needs), which is why
//! it's plainly `Serialize`/`Deserialize` with no registry/context
//! required to read it back.
//!
//! This crate doesn't fix the id type itself - deleting an action must
//! not shift or leak other actions' value slots, so a backend should
//! pick something with the same "stable across removal" guarantee a
//! generational key or a `Uuid` gives, but the choice is the
//! backend's, not this crate's.

/// Implemented once per concrete value type `T` a backend's
/// [`ValuePool`](crate::backend::SceneBackend::ValuePool) stores,
/// addressed by that backend's [`SceneBackend::ValueId`](crate::backend::SceneBackend::ValueId).
///
/// Not a wrapper around `T` and not an enum variant - just "here is
/// the real column for `T`," so `SceneRegistry::register_field`/
/// `register_op` can fetch or insert a `T` without knowing the pool's
/// concrete field layout.
pub trait ValueColumn<Id, T> {
    /// Returns a reference to the value at `id`, or `None` if `id`
    /// is stale (removed, or from a different pool).
    fn get(&self, id: Id) -> Option<&T>;

    /// Returns a mutable reference to the value at `id`, or `None` if
    /// `id` is stale.
    fn get_mut(&mut self, id: Id) -> Option<&mut T>;

    /// Inserts `value`, returning the id to reach it again.
    fn insert(&mut self, value: T) -> Id;
}
