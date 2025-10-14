use bevy::prelude::*;

use motiongfx_core::prelude::*;

pub trait TransformMotion<const N: usize> {
    fn transform(&mut self) -> TransformMotionBuilder;
}

impl<const N: usize, T: GetMutValue<Transform, N>> TransformMotion<N> for (Entity, T) {
    fn transform(&mut self) -> TransformMotionBuilder {
        TransformMotionBuilder::new(self.id(), self.1.get_mut_value())
    }
}

pub struct TransformMotionBuilder<'a> {
    id: Entity,
    pub transform: &'a mut Transform,
}

impl<'a> TransformMotionBuilder<'a> {
    pub fn new(id: Entity, transform: &'a mut Transform) -> Self {
        Self { id, transform }
    }

    pub fn to(&mut self, transfrom: Transform) -> Action<Transform, Transform> {
        act!(
            (self.id, Transform),
            start = { *self.transform },
            end = transfrom,
        )
    }

    pub fn to_translation(&mut self, translation: Vec3) -> Action<Vec3, Transform> {
        act!(
            (self.id, Transform),
            start = { self.transform }.translation,
            end = translation,
        )
    }

    pub fn to_translation_x(&mut self, x: f32) -> Action<f32, Transform> {
        act!(
            (self.id, Transform),
            start = { self.transform }.translation.x,
            end = x,
        )
    }

    pub fn to_translation_y(&mut self, y: f32) -> Action<f32, Transform> {
        act!(
            (self.id, Transform),
            start = { self.transform }.translation.y,
            end = y,
        )
    }

    pub fn to_translation_z(&mut self, z: f32) -> Action<f32, Transform> {
        act!(
            (self.id, Transform),
            start = { self.transform }.translation.z,
            end = z,
        )
    }

    pub fn to_scale(&mut self, scale: Vec3) -> Action<Vec3, Transform> {
        act!(
            (self.id, Transform),
            start = { self.transform }.scale,
            end = scale,
        )
    }

    pub fn to_scale_x(&mut self, x: f32) -> Action<f32, Transform> {
        act!(
            (self.id, Transform),
            start = { self.transform }.scale.x,
            end = x,
        )
    }

    pub fn to_scale_y(&mut self, y: f32) -> Action<f32, Transform> {
        act!(
            (self.id, Transform),
            start = { self.transform }.scale.y,
            end = y,
        )
    }

    pub fn to_scale_z(&mut self, z: f32) -> Action<f32, Transform> {
        act!(
            (self.id, Transform),
            start = { self.transform }.scale.z,
            end = z,
        )
    }

    pub fn to_rotation(&mut self, rotation: Quat) -> Action<Quat, Transform> {
        act!(
            (self.id, Transform),
            start = { self.transform }.rotation,
            end = rotation,
        )
    }
}
