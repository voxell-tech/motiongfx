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
//! `V` is the opaque value representation for subject state and action
//! arguments; the backend picks its shape (e.g. `Box<dyn Reflect>` for
//! Bevy). This crate only requires `V: Serialize + Deserialize`.

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_code)]

extern crate alloc;

pub mod block;
pub mod compile;
pub mod error;
pub mod refs;
pub mod registry;
pub mod scene;

pub mod prelude {
    pub use crate::block::{ActionCmd, Block, Combinator, Node};
    pub use crate::error::CompileError;
    pub use crate::refs::{
        EaseRef, FieldRef, InterpRef, OpRef, TypeName,
    };
    pub use crate::scene::{Scene, Stage, Subject};
}
