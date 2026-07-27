//! Bundles a backend's chosen types behind one associated-type trait,
//! so [`Scene`](crate::scene::Scene), [`SceneRegistry`](crate::registry::SceneRegistry),
//! [`ActionCmd`](crate::block::ActionCmd), and
//! [`CompileError`](crate::error::CompileError) only need a single
//! generic parameter (`<B>`) instead of separately threading
//! `Id`/`Value`/`OpId`/`InterpId`/`EaseId`/`World`.

use motiongfx::subject::SubjectId;

use crate::refs::Key;

/// A backend's chosen types for the scene format and registry.
///
/// Implement this once per backend - a zero-sized marker type is
/// enough - rather than repeating its `Id`/`Value`/`OpId`/`InterpId`/
/// `EaseId`/`World` choices at every `Scene`/`SceneRegistry` use site.
///
/// `Debug`/`Clone`/`PartialEq` aren't needed as supertraits: types
/// that bundle `B` (`Scene`, `ActionCmd`, ...) derive those via
/// `educe`, which infers bounds from the actual associated types used
/// (`B::Id`, `B::Value`, ...) rather than requiring `B` itself to
/// implement them. `'static` is needed, though - `SceneRegistry`
/// stores `Box<dyn FieldResolver<B> + ...>`, and a boxed trait object
/// defaults to a `'static` bound on its type parameters.
pub trait SceneBackend: 'static {
    /// The subject identifier type.
    type Id: SubjectId;
    /// The opaque value representation for subject state and action
    /// arguments.
    type Value: 'static;
    /// A registered action op's identifier.
    type OpId: Key;
    /// A registered interpolation function's identifier.
    type InterpId: Key;
    /// A registered easing function's identifier.
    type EaseId: Key;
    /// The runtime's world type.
    type World: 'static;
}
