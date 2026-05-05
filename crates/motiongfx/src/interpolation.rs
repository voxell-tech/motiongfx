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
                (*a) * (1.0 - t) + (*b) * t
            }
        }
    };
}

impl_float_interpolation!(f32, f32);
impl_float_interpolation!(f64, f64);
