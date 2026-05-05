#![no_std]

pub mod interpolation;
pub mod trace;

pub mod prelude {
    pub use crate::Peniko;
    pub use peniko;
    pub use peniko::kurbo;
}

/// Marker for [`Interpolation<Peniko>`] impls on [`peniko`] types.
///
/// [`Interpolation<Peniko>`]: motiongfx::interpolation::Interpolation
pub struct Peniko;
