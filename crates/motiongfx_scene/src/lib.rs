//! A serializable [`Scene`](scene::Scene), compiled into a runtime
//! [`Timeline`](motiongfx::timeline::Timeline). The scene is the source
//! of truth; the timeline is a derived view. See
//! `docs/scene-serialization.md` for the full design.
//!
//! ## Layers
//!
//! - **Format** (this crate): pure data, engine- and backend-agnostic.
//! - **Registry** (`registry`): reconstructs typed closures from the
//!   format's names, filled by the app.
//! - **Runtime** ([`motiongfx`]): `Timeline` / `Track` / combinators.
//!
//! [`SceneBackend::ValuePool`](backend::SceneBackend::ValuePool) holds
//! subject-state and action-argument values; the backend picks its
//! shape (a plain struct of named `sparse_map::SparseMap<T>` columns,
//! one per value type it needs - see [`value`]).

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_code)]

extern crate alloc;

pub mod backend;
pub mod block;
pub mod compile;
mod duration;
pub mod error;
pub mod refs;
pub mod registry;
pub mod scene;
pub mod value;

pub mod prelude {
    pub use crate::backend::{Key, SceneBackend, Storable};
    pub use crate::block::{ActionCmd, Block, Combinator, Node};
    pub use crate::error::CompileError;
    pub use crate::refs::{FieldRef, TypeName};
    pub use crate::scene::{Scene, Stage, Subject};
    pub use crate::value::{ValueColumn, ValueId};
}
