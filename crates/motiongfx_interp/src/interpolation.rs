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

/// Interpolation for integer types, walked through `f64` and rounded
/// back rather than kept exact.
///
/// `f64` holds every `i32`/`u32` exactly, so the endpoints come back
/// unchanged at `t == 0.0` and `t == 1.0` even past `2^24`.
#[macro_export]
macro_rules! impl_int_interpolation {
    ($ty:ty) => {
        $crate::impl_int_interpolation!($ty, ());
    };

    ($ty:ty, $marker:ty) => {
        impl $crate::interpolation::Interpolation<$marker> for $ty {
            #[inline]
            fn interp(a: &Self, b: &Self, t: f32) -> Self {
                if t == 0.0 {
                    return *a;
                }
                if t == 1.0 {
                    return *b;
                }
                let (a, b) = (*a as f64, *b as f64);
                (a + (b - a) * t as f64).round() as Self
            }
        }
    };
}

impl_int_interpolation!(i32);
impl_int_interpolation!(u32);
impl_int_interpolation!(u8);

#[cfg(test)]
mod tests {
    use crate::interpolation::Interpolation;

    fn lerp<T: Interpolation<()>>(a: T, b: T, t: f32) -> T {
        T::interp(&a, &b, t)
    }

    #[test]
    fn large_i32_endpoints_survive_the_boundaries() {
        let v = 100_000_007_i32; // past 2^24
        assert_eq!(lerp(v, v, 0.0), v);
        assert_eq!(lerp(v, v, 1.0), v);
    }

    #[test]
    fn large_u32_endpoints_survive_the_boundaries() {
        let v = 4_000_000_001_u32; // past 2^24 and past i32::MAX
        assert_eq!(lerp(v, v, 0.0), v);
        assert_eq!(lerp(v, v, 1.0), v);
    }
}
