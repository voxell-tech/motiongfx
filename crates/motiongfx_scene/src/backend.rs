//! Bundles a backend's chosen types behind one associated-type trait,
//! so [`Scene`](crate::scene::Scene), [`SceneRegistry`](crate::registry::SceneRegistry),
//! [`ActionCmd`](crate::block::ActionCmd), and
//! [`CompileError`](crate::error::CompileError) only need a single
//! generic parameter (`<B>`) instead of separately threading
//! `Id`/`Value`/`OpId`/`InterpId`/`EaseId`/`World`.

use serde::Serialize;
use serde::de::DeserializeOwned;

use motiongfx::subject::SubjectId;

use crate::refs::Key;

/// A backend's chosen types for the scene format and registry.
///
/// Implement this once per backend - a zero-sized marker type is
/// enough - rather than repeating its `Id`/`Value`/`OpId`/`InterpId`/
/// `EaseId`/`World` choices at every `Scene`/`SceneRegistry` use site.
///
/// `Serialize + DeserializeOwned` on every field type but `World`:
/// serializing a [`Scene`](crate::scene::Scene) is the whole point of
/// this crate, so every backend must provide it. `'static` is needed
/// because `SceneRegistry` stores `Box<dyn FieldResolver<B> + ...>`,
/// which defaults trait objects to a `'static` bound.
pub trait SceneBackend: 'static {
    /// The subject identifier type.
    type Id: SubjectId + Serialize + DeserializeOwned;
    /// The opaque value representation for subject state and action
    /// arguments.
    type Value: 'static + Serialize + DeserializeOwned;
    /// A registered action op's identifier.
    type OpId: Key + Serialize + DeserializeOwned;
    /// A registered interpolation function's identifier.
    type InterpId: Key + Serialize + DeserializeOwned;
    /// A registered easing function's identifier.
    type EaseId: Key + Serialize + DeserializeOwned;
    /// The runtime's world type.
    type World: 'static;
}
