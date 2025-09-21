use bevy_color::prelude::*;
use bevy_math::*;
use bevy_transform::components::Transform;

/// Trait for interpolating between 2 values based on a f32 `t` value.
pub trait Interpolation<T = Self, U = Self> {
    /// Linearly interpolate between 2 values based on a f32 `t` value.
    fn interp(&self, rhs: &T, t: f32) -> U;
}

macro_rules! impl_animatable {
    ($ty:ty) => {
        impl Interpolation for $ty {
            #[inline]
            fn interp(&self, rhs: &Self, t: f32) -> Self {
                ::bevy::animation::animatable::Animatable::interpolate(
                    self, rhs, t,
                )
            }
        }
    };
}

macro_rules! impl_stable_interpolate {
    ($ty:ty) => {
        impl Interpolation for $ty {
            #[inline]
            fn interp(&self, rhs: &Self, t: f32) -> Self {
                ::bevy_math::common_traits::StableInterpolate::interpolate_stable(self, rhs, t)
            }
        }
    };
}

// Maths.
impl_animatable!(bool);
impl_animatable!(f32);
impl_animatable!(Vec2);
impl_animatable!(Vec3);
impl_animatable!(Vec3A);
impl_animatable!(Vec4);
impl_animatable!(Quat);
impl_animatable!(f64);
impl_animatable!(DVec2);
impl_animatable!(DVec3);
impl_animatable!(DVec4);

impl Interpolation for DQuat {
    fn interp(&self, rhs: &Self, t: f32) -> Self {
        self.slerp(*rhs, t as f64)
    }
}

impl Interpolation for u8 {
    fn interp(&self, rhs: &Self, t: f32) -> Self {
        let other = *rhs as f32;
        let self_ = *self as f32;

        ((other - self_) * t + self_) as u8
    }
}

// Directions
impl_stable_interpolate!(Rot2);
impl_stable_interpolate!(Dir2);
impl_stable_interpolate!(Dir3);
impl_stable_interpolate!(Dir3A);

// Colors.
impl_animatable!(LinearRgba);
impl_animatable!(Laba);
impl_animatable!(Oklaba);
// impl_animatable!(Srgba);
impl_animatable!(Xyza);

impl Interpolation for Color {
    #[inline]
    fn interp(&self, rhs: &Self, t: f32) -> Self {
        Color::mix(self, rhs, t)
    }
}

impl Interpolation for Srgba {
    fn interp(&self, rhs: &Self, t: f32) -> Self {
        self.lerp(*rhs, t)
    }
}

// Components.
impl Interpolation for Transform {
    fn interp(&self, rhs: &Self, t: f32) -> Self {
        Self {
            translation: self.translation.interp(&rhs.translation, t),
            rotation: self.rotation.interp(&rhs.rotation, t),
            scale: self.scale.interp(&rhs.scale, t),
        }
    }
}
