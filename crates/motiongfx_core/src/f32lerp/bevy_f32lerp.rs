use bevy::prelude::*;

use super::F32Lerp;

impl F32Lerp for Color {
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        Color::mix(self, rhs, t)
    }
}

impl F32Lerp for LinearRgba {
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        LinearRgba::mix(self, rhs, t)
    }
}

impl F32Lerp for Transform {
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        Self {
            translation: Vec3::f32lerp(&self.translation, &rhs.translation, t),
            rotation: Quat::f32lerp(&self.rotation, &rhs.rotation, t),
            scale: Vec3::f32lerp(&self.scale, &rhs.scale, t),
        }
    }
}
