//! Compilation error types for the scene-to-timeline pipeline.

use core::fmt;

use crate::refs::{EaseRef, FieldRef, InterpRef, OpRef};

/// Errors that can occur during [`compile`](crate::compile()) of a
/// scene into a runtime [`Timeline`](motiongfx::timeline::Timeline).
///
/// Each variant pinpoints the scene element that couldn't be resolved.
#[derive(Debug, Clone, PartialEq)]
pub enum CompileError<Id> {
    /// The scene references a subject id that was never registered in the
    /// registry's subject map.
    UnknownSubject(Id),

    /// The field reference has no matching `UntypedField` in the registry.
    UnknownField(FieldRef),

    /// No op builder was registered for `OpRef` under this value type.
    UnknownOp(&'static str, OpRef),

    /// No easing function registered under this name.
    UnknownEase(EaseRef),

    /// No interpolation function registered under this name.
    UnknownInterp(InterpRef),

    /// The op builder's expected type doesn't match what's registered
    /// for the field: a registration bug.
    TypeMismatch {
        type_name: &'static str,
        field: FieldRef,
    },
}

// Manual Display to keep the crate `no_std` + `alloc`.
impl<Id: fmt::Debug> fmt::Display for CompileError<Id> {
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
                write!(f, "unknown op {} for type {type_name}", op.0)
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
