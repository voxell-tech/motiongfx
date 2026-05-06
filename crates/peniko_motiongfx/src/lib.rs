#![no_std]

pub mod interpolation;
pub mod trace;

pub mod prelude {
    pub use peniko;
    pub use peniko::kurbo;

    pub use crate::Peniko;
    pub use crate::trace::{
        CubicTracer, LineTracer, PathTracer, QuadTracer, Trace,
    };
}

/// Marker for [`Interpolation<Peniko>`] impls on [`peniko`] types.
///
/// [`Interpolation<Peniko>`]: motiongfx::interpolation::Interpolation
pub struct Peniko;
