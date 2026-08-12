/// Trait for interpolating between two values.
///
/// The `M` marker parameter exists solely to satisfy the orphan rule:
/// downstream crates can provide a local marker type to implement
/// this trait for foreign `Self` types.
pub trait Interpolation<M> {
    fn interp(a: &Self, b: &Self, t: f32) -> Self;
}

#[macro_export]
macro_rules! impl_float_interpolation {
    ($ty:ty, $base:ty) => {
        $crate::impl_float_interpolation!($ty, $base, ());
    };

    ($ty:ty, $base:ty, $marker:ty) => {
        impl $crate::interpolation::Interpolation<$marker> for $ty {
            #[inline]
            fn interp(a: &Self, b: &Self, t: f32) -> Self {
                let t = <$base>::from(t);
                (*a) + (*b - *a) * t
            }
        }
    };
}

impl_float_interpolation!(f32, f32);
impl_float_interpolation!(f64, f64);

/// Interpolation for integer types, walked through `f32` and rounded
/// back rather than kept exact.
#[macro_export]
macro_rules! impl_int_interpolation {
    ($ty:ty) => {
        $crate::impl_int_interpolation!($ty, ());
    };

    ($ty:ty, $marker:ty) => {
        impl $crate::interpolation::Interpolation<$marker> for $ty {
            #[inline]
            fn interp(a: &Self, b: &Self, t: f32) -> Self {
                let (a, b) = (*a as f32, *b as f32);
                (a + (b - a) * t) as Self
            }
        }
    };
}

impl_int_interpolation!(i32);
impl_int_interpolation!(u32);
impl_int_interpolation!(u8);
