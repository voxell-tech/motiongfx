use bevy::math::{prelude::*, DQuat, DVec2, DVec3, DVec4};

use super::F32Lerp;

impl F32Lerp for Vec2 {
    #[inline]
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        Vec2::lerp(*self, *rhs, t)
    }
}

impl F32Lerp for Vec3 {
    #[inline]
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        Vec3::lerp(*self, *rhs, t)
    }
}

impl F32Lerp for Vec4 {
    #[inline]
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        Vec4::lerp(*self, *rhs, t)
    }
}

impl F32Lerp for Quat {
    #[inline]
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        Quat::lerp(*self, *rhs, t)
    }
}

impl F32Lerp for DVec2 {
    #[inline]
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        DVec2::lerp(*self, *rhs, t as f64)
    }
}

impl F32Lerp for DVec3 {
    #[inline]
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        DVec3::lerp(*self, *rhs, t as f64)
    }
}

impl F32Lerp for DVec4 {
    #[inline]
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        DVec4::lerp(*self, *rhs, t as f64)
    }
}

impl F32Lerp for DQuat {
    #[inline]
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        DQuat::lerp(*self, *rhs, t as f64)
    }
}
