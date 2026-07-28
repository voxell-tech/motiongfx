//! Per-backend value storage.
//!
//! `ActionCmd`/`Subject` reference a [`ValueId`], resolved through the
//! backend's own [`SceneBackend::ValuePool`](crate::backend::SceneBackend::ValuePool)
//! via [`ValueColumn`] - one small impl per concrete value type. No
//! wrapper type around the value, no enum: a backend's pool is a plain
//! struct of named [`sparse_map::SparseMap`] columns (one per value
//! type it needs), which is why it's plainly `Serialize`/`Deserialize`
//! with no registry/context required to read it back.

/// A stable, generational reference into a [`SceneBackend::ValuePool`](crate::backend::SceneBackend::ValuePool).
///
/// Re-exported from `sparse_map` rather than a crate-local newtype:
/// deleting an action must not shift or leak other actions' value
/// slots, which is exactly what `sparse_map::Key`'s generation
/// counter already guarantees on removal/reuse.
pub use sparse_map::Key as ValueId;

/// Implemented once per concrete value type `T` a backend's
/// [`ValuePool`](crate::backend::SceneBackend::ValuePool) stores.
///
/// Not a wrapper around `T` and not an enum variant - just "here is
/// the real [`SparseMap<T>`](sparse_map::SparseMap) column," so
/// `SceneRegistry::register_field`/`register_op` can fetch or insert a
/// `T` without knowing the pool's concrete field layout.
pub trait ValueColumn<T> {
    /// Returns a reference to the value at `id`, or `None` if `id`
    /// is stale (removed, or from a different pool).
    fn get(&self, id: ValueId) -> Option<&T>;

    /// Returns a mutable reference to the value at `id`, or `None` if
    /// `id` is stale.
    fn get_mut(&mut self, id: ValueId) -> Option<&mut T>;

    /// Inserts `value`, returning the [`ValueId`] to reach it again.
    fn insert(&mut self, value: T) -> ValueId;
}
