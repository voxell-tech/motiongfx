use bevy_math::*;

/// Trait for interpolating between 2 values based on a f32 `t` value.
pub trait Interpolation<T = Self, U = Self> {
    /// Linearly interpolate between 2 values based on a f32 `t` value.
    fn interp(a: &Self, b: &T, t: f32) -> U;
}

#[macro_export]
macro_rules! impl_float_interpolation {
    ($ty:ty, $base:ty) => {
        impl $crate::interpolation::Interpolation for $ty {
            #[inline]
            fn interp(a: &Self, b: &Self, t: f32) -> Self {
                let t = <$base>::from(t);
                (*a) * (1.0 - t) + (*b) * t
            }
        }
    };
}

macro_rules! impl_slerp_interpolation {
    ($ty: ty, $base: ty) => {
        impl $crate::interpolation::Interpolation for $ty {
            #[inline]
            fn interp(a: &Self, b: &Self, t: f32) -> Self {
                let t = <$base>::from(t);
                a.slerp(*b, t)
            }
        }
    };
}

macro_rules! impl_step_interpolation {
    ($ty: ty) => {
        impl $crate::interpolation::Interpolation for $ty {
            #[inline]
            fn interp(a: &Self, b: &Self, t: f32) -> Self {
                $crate::interpolation::step(*a, *b, t)
            }
        }
    };
}

impl_step_interpolation!(bool);

impl_float_interpolation!(f32, f32);
impl_float_interpolation!(Vec2, f32);
impl_float_interpolation!(Vec3, f32);
impl_float_interpolation!(Vec3A, f32);
impl_float_interpolation!(Vec4, f32);

impl_float_interpolation!(f64, f64);
impl_float_interpolation!(DVec2, f64);
impl_float_interpolation!(DVec3, f64);
impl_float_interpolation!(DVec4, f64);

impl_slerp_interpolation!(Quat, f32);
impl_slerp_interpolation!(DQuat, f64);
impl_slerp_interpolation!(Rot2, f32);
impl_slerp_interpolation!(Dir2, f32);
impl_slerp_interpolation!(Dir3, f32);
impl_slerp_interpolation!(Dir3A, f32);

impl Interpolation for u8 {
    fn interp(a: &Self, b: &Self, t: f32) -> Self {
        let a = *a as f32;
        let b = *b as f32;

        ((b - a) * t + a) as u8
    }
}

#[cfg(feature = "color")]
pub mod color {
    use bevy_color::prelude::*;

    use super::Interpolation;

    macro_rules! impl_color_interpolation {
        ($ty:ty) => {
            impl $crate::interpolation::Interpolation for $ty {
                #[inline]
                fn interp(a: &Self, b: &Self, t: f32) -> Self {
                    (*a) * (1.0 - t) + (*b) * t
                }
            }
        };
    }

    impl_color_interpolation!(LinearRgba);
    impl_color_interpolation!(Laba);
    impl_color_interpolation!(Oklaba);
    impl_color_interpolation!(Srgba);
    impl_color_interpolation!(Xyza);

    impl Interpolation for Color {
        #[inline]
        fn interp(a: &Self, b: &Self, t: f32) -> Self {
            Color::mix(a, b, t)
        }
    }
}

#[cfg(feature = "transform")]
pub mod transform {
    use bevy_transform::components::Transform;

    use super::Interpolation;

    impl Interpolation for Transform {
        fn interp(a: &Self, b: &Self, t: f32) -> Self {
            Self {
                translation: Interpolation::interp(
                    &a.translation,
                    &b.translation,
                    t,
                ),
                rotation: Interpolation::interp(
                    &a.rotation,
                    &b.rotation,
                    t,
                ),
                scale: Interpolation::interp(&a.scale, &b.scale, t),
            }
        }
    }
}

/// Steps between two different discrete values of any type.
/// Returns `a` if `t < 1.0`, otherwise returns `b`.
#[inline]
pub fn step<T>(a: T, b: T, t: f32) -> T {
    if t < 1.0 {
        a
    } else {
        b
    }
}
