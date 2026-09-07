#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]
#![deny(unsafe_code)]

extern crate alloc;

pub mod backend;
pub mod block;
pub mod compile;
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
    pub use crate::scene::{FieldSeed, Scene, Stage, Subject};
    pub use crate::value::ValueColumn;
}

pub use motiongfx;
