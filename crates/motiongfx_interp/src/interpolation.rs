/// Function for interpolating a type based on a [`f32`] time.
pub type InterpFn<T> = fn(start: &T, end: &T, t: f32) -> T;

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

/// Implements [`Interpolation`] for a built-in integer type under a
/// downstream marker, reusing the crate's own `Interpolation<()>` impl.
#[macro_export]
macro_rules! impl_int_interpolation {
    ($ty:ty, $marker:ty) => {
        impl $crate::interpolation::Interpolation<$marker> for $ty {
            #[inline]
            fn interp(a: &Self, b: &Self, t: f32) -> Self {
                <$ty as $crate::interpolation::Interpolation<()>>::interp(
                    a, b, t,
                )
            }
        }
    };
}

/// Integer interpolation that stays in the type's own width.
///
/// Exact at both ends, monotonic in `t`, bounded for `t` in `0..=1`,
/// and symmetric: swapping `a` and `b` with `1 - t` gives the same
/// value. The offset is measured from the lower endpoint and only it
/// goes through `f64`, so large `i64`/`u64` values keep their
/// precision. Overshooting eases saturate at the type's bounds.
macro_rules! builtin_int_interpolation {
    ($ty:ty, $dist:ty, $add:ident, $sub:ident) => {
        impl Interpolation<()> for $ty {
            #[inline]
            fn interp(a: &Self, b: &Self, t: f32) -> Self {
                let (a, b) = (*a, *b);
                let lo = a.min(b);
                let dist = a.abs_diff(b);
                // Measure from the lower end: `t` for `a <= b`, `1 - t`
                // otherwise.
                let t =
                    if a <= b { t as f64 } else { 1.0 - t as f64 };
                if t == 1.0 {
                    return a.max(b);
                }
                let offset = crate::ops::round(dist as f64 * t);
                if offset < 0.0 {
                    return lo.$sub(-offset as $dist);
                }
                // `dist as f64` may round past `dist`, so clamp to keep
                // each side of `t == 1` on its side of the far end.
                let offset = offset as $dist;
                lo.$add(if t < 1.0 {
                    offset.min(dist)
                } else {
                    offset.max(dist)
                })
            }
        }
    };
}

builtin_int_interpolation!(u8, u8, saturating_add, saturating_sub);
builtin_int_interpolation!(u32, u32, saturating_add, saturating_sub);
builtin_int_interpolation!(u64, u64, saturating_add, saturating_sub);
builtin_int_interpolation!(
    usize,
    usize,
    saturating_add,
    saturating_sub
);
builtin_int_interpolation!(
    i32,
    u32,
    saturating_add_unsigned,
    saturating_sub_unsigned
);
builtin_int_interpolation!(
    i64,
    u64,
    saturating_add_unsigned,
    saturating_sub_unsigned
);

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

    #[test]
    fn marker_impl_matches_builtin() {
        struct Marker;
        crate::impl_int_interpolation!(u8, Marker);

        let v = <u8 as Interpolation<Marker>>::interp(&10, &200, 0.3);
        assert_eq!(v, lerp(10_u8, 200, 0.3));
    }

    #[test]
    fn endpoints_are_exact() {
        const BIG: u64 = (1 << 53) + 3; // `BIG as f64` rounds up
        assert_eq!(lerp(0_u64, BIG, 0.0), 0);
        assert_eq!(lerp(0_u64, BIG, 1.0), BIG);
        assert_eq!(lerp(BIG, 0_u64, 0.0), BIG);
        assert_eq!(lerp(BIG, 0_u64, 1.0), 0);
        assert_eq!(lerp(i64::MIN, i64::MAX, 0.0), i64::MIN);
        assert_eq!(lerp(i64::MIN, i64::MAX, 1.0), i64::MAX);
    }

    #[test]
    fn equal_endpoints_stay_put() {
        for t in [-2.0, -0.5, 0.0, 0.3, 1.0, 7.0] {
            assert_eq!(lerp(42_u8, 42, t), 42);
            assert_eq!(lerp(-9_i64, -9, t), -9);
        }
    }

    #[test]
    fn reversed_endpoints_land_on_same_value() {
        assert_eq!(lerp(0_u8, 5, 0.5), lerp(5_u8, 0, 0.5));
        assert_eq!(lerp(-7_i32, 2, 0.25), lerp(2_i32, -7, 0.75));
        for i in 0..=1024 {
            let t = i as f32 / 1024.0;
            let s = 1.0 - t; // exact for these `t`
            assert_eq!(lerp(3_u8, 250, t), lerp(250_u8, 3, s));
            assert_eq!(
                lerp(-1000_i32, 999, t),
                lerp(999_i32, -1000, s)
            );
        }
    }

    #[test]
    fn sweep_is_monotonic_and_bounded() {
        fn check<T>(a: T, b: T)
        where
            T: Interpolation<()> + Copy + Ord + core::fmt::Debug,
        {
            let (lo, hi) = (a.min(b), a.max(b));
            let mut prev = lerp(a, b, -0.5);
            for i in -512..=1536 {
                let t = i as f32 / 1024.0;
                let v = lerp(a, b, t);
                if a <= b {
                    assert!(v >= prev, "{a:?}->{b:?} at {t}");
                } else {
                    assert!(v <= prev, "{a:?}->{b:?} at {t}");
                }
                if (0.0..=1.0).contains(&t) {
                    assert!(
                        lo <= v && v <= hi,
                        "{a:?}->{b:?} at {t}"
                    );
                }
                prev = v;
            }
        }
        check(0_u8, 255);
        check(255_u8, 0);
        check(-7_i32, 2);
        check(i64::MIN, i64::MAX);
        check(u64::MAX, 0);
    }

    #[test]
    fn tiny_t_on_reversed_large_range_stays_at_start() {
        // `1 - t` rounds to exactly `1` here, and `BIG as f64` rounds
        // past `BIG`, so the far end must come from `max(a, b)`.
        const BIG: u64 = (1 << 53) + 3;
        let tiny = f32::from_bits(1);
        assert_eq!(lerp(BIG, 0, tiny), BIG);
        assert_eq!(lerp(BIG, 0, -tiny), BIG);
    }

    #[test]
    fn large_i64_midpoint_keeps_precision() {
        let a = 9_007_199_254_740_993_i64; // 2^53 + 1
        assert_eq!(lerp(a, a + 4, 0.5), a + 2);
    }
}
