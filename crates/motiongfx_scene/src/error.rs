//! Compilation error types for the scene-to-timeline pipeline.

use core::fmt;

use educe::Educe;

use crate::backend::SceneBackend;
use crate::refs::FieldRef;

/// Errors that can occur during [`compile`](crate::compile::compile())
/// of a scene into a runtime [`Timeline`](motiongfx::timeline::Timeline).
///
/// Each variant pinpoints the scene element that couldn't be resolved.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq)]
pub enum CompileError<B: SceneBackend> {
    /// The scene references a subject id that was never registered in the
    /// registry's subject map.
    UnknownSubject(B::Id),

    /// The field reference has no matching
    /// [`UntypedField`](motiongfx::field_path::field::UntypedField) in
    /// the registry.
    UnknownField(FieldRef),

    /// No op builder was registered for this
    /// [`Key`](crate::refs::Key) under this value type.
    UnknownOp(&'static str, B::OpId),

    /// No easing function registered under this
    /// [`Key`](crate::refs::Key).
    UnknownEase(B::EaseId),

    /// No interpolation function registered under this
    /// [`Key`](crate::refs::Key).
    UnknownInterp(B::InterpId),

    /// The op builder's expected type doesn't match what's registered
    /// for the field: a registration bug.
    TypeMismatch {
        type_name: &'static str,
        field: FieldRef,
    },
}

// Manual Display to keep the crate `no_std` + `alloc`.
impl<B: SceneBackend> fmt::Display for CompileError<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSubject(id) => {
                write!(f, "unknown subject {id:?}")
            }
            Self::UnknownField(field) => {
                write!(
                    f,
                    "unknown field {}::{}",
                    field.type_name, field.path
                )
            }
            Self::UnknownOp(type_name, op) => {
                write!(f, "unknown op {op:?} for type {type_name}")
            }
            Self::UnknownEase(ease) => {
                write!(f, "unknown easing function {ease:?}")
            }
            Self::UnknownInterp(interp) => {
                write!(f, "unknown interpolation function {interp:?}")
            }
            Self::TypeMismatch { type_name, field } => {
                write!(
                    f,
                    "type mismatch for {type_name} field {}::{}",
                    field.type_name, field.path
                )
            }
        }
    }
}
