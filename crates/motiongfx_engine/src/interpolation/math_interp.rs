use bevy::math::prelude::*;
use bevy::math::{DQuat, DVec2, DVec3, DVec4};

use super::Interpolation;

// impl Interpolation for Vec2 {
//     #[inline]
//     fn interp(&self, rhs: &Self, t: f32) -> Self {
//         Vec2::lerp(*self, *rhs, t)
//     }
// }

// impl Interpolation for Vec3 {
//     #[inline]
//     fn interp(&self, rhs: &Self, t: f32) -> Self {
//         Vec3::lerp(*self, *rhs, t)
//     }
// }

// impl Interpolation for Vec4 {
//     #[inline]
//     fn interp(&self, rhs: &Self, t: f32) -> Self {
//         Vec4::lerp(*self, *rhs, t)
//     }
// }

// impl Interpolation for Quat {
//     #[inline]
//     fn interp(&self, rhs: &Self, t: f32) -> Self {
//         Quat::lerp(*self, *rhs, t)
//     }
// }

// impl Interpolation for DVec2 {
//     #[inline]
//     fn interp(&self, rhs: &Self, t: f32) -> Self {
//         DVec2::lerp(*self, *rhs, t as f64)
//     }
// }

// impl Interpolation for DVec3 {
//     #[inline]
//     fn interp(&self, rhs: &Self, t: f32) -> Self {
//         DVec3::lerp(*self, *rhs, t as f64)
//     }
// }

// impl Interpolation for DVec4 {
//     #[inline]
//     fn interp(&self, rhs: &Self, t: f32) -> Self {
//         DVec4::lerp(*self, *rhs, t as f64)
//     }
// }

// impl Interpolation for DQuat {
//     #[inline]
//     fn interp(&self, rhs: &Self, t: f32) -> Self {
//         DQuat::lerp(*self, *rhs, t as f64)
//     }
// }
