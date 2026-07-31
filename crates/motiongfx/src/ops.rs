//! `f32` math used by [`crate::ease`]: `std`'s intrinsics when
//! available, `libm` otherwise. Replaces `bevy_math::ops`, which
//! pulled in `glam`/`itertools` for four functions.

#[cfg(feature = "std")]
#[inline]
pub fn sin(x: f32) -> f32 {
    x.sin()
}

#[cfg(not(feature = "std"))]
#[inline]
pub fn sin(x: f32) -> f32 {
    libm::sinf(x)
}

#[cfg(feature = "std")]
#[inline]
pub fn cos(x: f32) -> f32 {
    x.cos()
}

#[cfg(not(feature = "std"))]
#[inline]
pub fn cos(x: f32) -> f32 {
    libm::cosf(x)
}

#[cfg(feature = "std")]
#[inline]
pub fn sqrt(x: f32) -> f32 {
    x.sqrt()
}

#[cfg(not(feature = "std"))]
#[inline]
pub fn sqrt(x: f32) -> f32 {
    libm::sqrtf(x)
}

#[cfg(feature = "std")]
#[inline]
pub fn powf(x: f32, y: f32) -> f32 {
    x.powf(y)
}

#[cfg(not(feature = "std"))]
#[inline]
pub fn powf(x: f32, y: f32) -> f32 {
    libm::powf(x, y)
}
