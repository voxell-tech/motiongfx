use bevy::prelude::*;

pub mod bevy_f32lerp;
pub mod math_f32lerp;

pub trait F32Lerp<T = Self, U = Self> {
    /// Lerp between 2 values based on a [`f32`] `t` value.
    fn f32lerp(&self, rhs: &T, t: f32) -> U;
}

impl F32Lerp for f32 {
    #[inline]
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        f32::lerp(*self, *rhs, t)
    }
}

impl F32Lerp for f64 {
    #[inline]
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        f64::lerp(*self, *rhs, t as f64)
    }
}

impl F32Lerp for u8 {
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        let other = *rhs as f32;
        let self_ = *self as f32;

        ((other - self_) * t + self_) as u8
    }
}
