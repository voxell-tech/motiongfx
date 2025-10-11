use bevy_math::*;
use motiongfx::prelude::*;
use motiongfx::subject::SubjectId;

pub trait ActionInterpTimelineExt {
    fn act_interp<I, S, T>(
        &mut self,
        target: I,
        field: Field<S, T>,
        action: impl Action<T>,
    ) -> InterpActionBuilder<'_, T>
    where
        I: SubjectId,
        S: 'static,
        T: Interpolation + ThreadSafe;
}

impl ActionInterpTimelineExt for TimelineBuilder {
    /// Add an [`Action`] with interpolation using
    /// [`Interpolation::interp`].
    fn act_interp<I, S, T>(
        &mut self,
        target: I,
        field: Field<S, T>,
        action: impl Action<T>,
    ) -> InterpActionBuilder<'_, T>
    where
        I: SubjectId,
        S: 'static,
        T: Interpolation + ThreadSafe,
    {
        self.act(target, field, action).with_interp(T::interp)
    }
}

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
