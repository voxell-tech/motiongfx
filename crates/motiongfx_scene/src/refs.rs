//! Stable, format-owned references.
//!
//! Everything the animation portion points at is a name or an id, never
//! a Rust type or a runtime handle. The [`registry`](crate::registry)
//! resolves these into concrete accessors, ops, and subjects at compile
//! time.

use alloc::boxed::Box;
use alloc::string::String;
use core::fmt;
use core::fmt::Debug;
use core::hash::Hash;

use serde::{Deserialize, Serialize};

use motiongfx::ThreadSafe;

/// An auto trait bound for any small `Copy` identifier usable as a
/// hashmap/table key - thread-safe, debuggable, and cheap to compare
/// and hash.
pub trait Key: ThreadSafe + Debug + Copy + Clone + Eq + Hash {}

impl<T> Key for T where
    T: ThreadSafe + Debug + Copy + Clone + Eq + Hash
{
}

/// A fully-qualified type name, e.g.
/// `"bevy_transform::components::transform::Transform"`.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct TypeName(Box<str>);

impl TypeName {
    pub fn new(name: impl Into<Box<str>>) -> Self {
        Self(name.into())
    }
}

impl fmt::Display for TypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for TypeName {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl From<String> for TypeName {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

/// Names a field to animate: its owning source type plus a field path
/// like `"translation::x"`.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct FieldRef {
    pub type_name: TypeName,
    pub path: Box<str>,
}
