#![no_std]

pub mod interpolation;
pub mod trace;

/// Marker type for [`Interpolation`] impls on kurbo geometry types.
pub struct Kurbo;
