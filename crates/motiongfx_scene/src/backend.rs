//! Bundles a backend's chosen types behind one associated-type trait,
//! so [`Scene`](crate::scene::Scene), [`SceneRegistry`](crate::registry::SceneRegistry),
//! [`ActionCmd`](crate::block::ActionCmd), and
//! [`CompileError`](crate::error::CompileError) only need a single
//! generic parameter (`<B>`) instead of separately threading
//! `Id`/`ValuePool`/`OpId`/`InterpId`/`EaseId`/`World`.

use core::fmt::Debug;
use core::hash::Hash;

use serde::Serialize;
use serde::de::DeserializeOwned;

use motiongfx::ThreadSafe;
use motiongfx::subject::SubjectId;

/// An auto trait bound for any small `Copy` identifier usable as a
/// hashmap/table key - thread-safe, debuggable, and cheap to compare
/// and hash.
pub trait Key: ThreadSafe + Debug + Copy + Clone + Eq + Hash {}

impl<T> Key for T where
    T: ThreadSafe + Debug + Copy + Clone + Eq + Hash
{
}

/// An auto trait bound for any type storable in the serialized
/// format.
pub trait Storable: Serialize + DeserializeOwned {}

impl<T> Storable for T where T: Serialize + DeserializeOwned {}

/// A backend's chosen types for the scene format and registry.
///
/// Implement this once per backend - a zero-sized marker type is
/// enough - rather than repeating its `Id`/`ValuePool`/`OpId`/
/// `InterpId`/`EaseId`/`World` choices at every `Scene`/`SceneRegistry`
/// use site.
///
/// [`Storable`] on every field type but `World`: serializing a
/// [`Scene`](crate::scene::Scene) is the whole point of this crate,
/// so every backend must provide it. `'static` is needed because
/// `SceneRegistry` stores `Box<dyn FieldResolver<B> + ...>`, which
/// defaults trait objects to a `'static` bound.
pub trait SceneBackend: 'static {
    /// The subject identifier type.
    type Id: SubjectId + Storable;
    /// Holds subject state and action-argument values, one
    /// [`ValueColumn<T>`](crate::value::ValueColumn) impl per concrete
    /// value type the backend needs. A plain struct of named
    /// `sparse_map::SparseMap<T>` fields, not a closed enum or a boxed
    /// trait object - see `crate::value` for why.
    type ValuePool: Storable + Default + 'static;
    /// A registered action op's identifier.
    type OpId: Key + Storable;
    /// A registered interpolation function's identifier.
    type InterpId: Key + Storable;
    /// A registered easing function's identifier.
    type EaseId: Key + Storable;
    /// The runtime's world type.
    type World: 'static;
}
