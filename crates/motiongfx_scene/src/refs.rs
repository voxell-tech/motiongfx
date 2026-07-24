//! Stable, format-owned references.
//!
//! Everything the animation portion points at is a name or an id, never
//! a Rust type or a runtime handle. The [`registry`](crate::registry)
//! resolves these into concrete accessors, ops, and subjects at compile
//! time.

use alloc::boxed::Box;
use alloc::string::String;
use core::fmt;

use serde::{Deserialize, Serialize};

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

/// Name of a registered action op, e.g. `"to"` (absolute) or `"by"`
/// (relative). Resolved to a value-into-closure builder by the registry.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct OpRef(pub String);

/// Name of a registered easing function, e.g. `"cubic::ease_in_out"`.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct EaseRef(Box<str>);

impl EaseRef {
    pub fn new(name: impl Into<Box<str>>) -> Self {
        Self(name.into())
    }
}

/// Name of a registered interpolation function.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct InterpRef(Box<str>);

impl InterpRef {
    pub fn new(name: impl Into<Box<str>>) -> Self {
        Self(name.into())
    }
}
