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

/// Interpolate an integer type by lerping in `f64` and rounding, so animated
/// integer fields (e.g. counts) step smoothly between values.
#[macro_export]
macro_rules! impl_int_interpolation {
    ($ty:ty) => {
        $crate::impl_int_interpolation!($ty, ());
    };

    ($ty:ty, $marker:ty) => {
        impl $crate::interpolation::Interpolation<$marker> for $ty {
            #[inline]
            fn interp(a: &Self, b: &Self, t: f32) -> Self {
                let a = *a as f64;
                let b = *b as f64;
                (a + (b - a) * t as f64).round() as $ty
            }
        }
    };
}

impl_int_interpolation!(i32);
impl_int_interpolation!(u32);
impl_int_interpolation!(i64);
impl_int_interpolation!(u64);
impl_int_interpolation!(usize);
