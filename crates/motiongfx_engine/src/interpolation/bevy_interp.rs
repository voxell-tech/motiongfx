use bevy::prelude::*;

use super::Interpolation;

// impl Interpolation for Color {
//     fn interp(&self, rhs: &Self, t: f32) -> Self {
//         Color::mix(self, rhs, t)
//     }
// }

// impl Interpolation for Srgba {
//     fn interp(&self, rhs: &Self, t: f32) -> Self {
//         Srgba::mix(self, rhs, t)
//     }
// }

// impl Interpolation for LinearRgba {
//     fn interp(&self, rhs: &Self, t: f32) -> Self {
//         LinearRgba::mix(self, rhs, t)
//     }
// }

// TODO: Implement for all colors.

// impl Interpolation for Transform {
//     fn interp(&self, rhs: &Self, t: f32) -> Self {
//         Self {
//             translation: Vec3::interp(
//                 &self.translation,
//                 &rhs.translation,
//                 t,
//             ),
//             rotation: Quat::interp(&self.rotation, &rhs.rotation, t),
//             scale: Vec3::interp(&self.scale, &rhs.scale, t),
//         }
//     }
// }
