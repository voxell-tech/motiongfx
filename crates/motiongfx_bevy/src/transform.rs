use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_transform::prelude::Transform;
use motiongfx_core::prelude::*;

pub struct Translation {
    target_id: Entity,
    translation: Vec3,
}

impl Translation {
    pub fn new(target_id: Entity, translation: Vec3) -> Self {
        Self {
            target_id,
            translation,
        }
    }

    pub fn from_transform(target_id: Entity, transform: &Transform) -> Self {
        Self {
            target_id,
            translation: transform.translation,
        }
    }

    pub fn translate(&mut self, translation: Vec3) -> Action<Transform, Vec3, EmptyRes> {
        let new_translation: Vec3 = self.translation + translation;

        let action: Action<Transform, Vec3, EmptyRes> = Action::new(
            self.target_id,
            self.translation,
            new_translation,
            Self::interp,
        );

        self.translation = new_translation;

        action
    }

    pub fn translate_to(&mut self, translation: Vec3) -> Action<Transform, Vec3, EmptyRes> {
        let action: Action<Transform, Vec3, EmptyRes> =
            Action::new(self.target_id, self.translation, translation, Self::interp);

        self.translation = translation;

        action
    }

    fn interp(
        transform: &mut Transform,
        begin: &Vec3,
        end: &Vec3,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        transform.translation = Vec3::lerp(*begin, *end, t);
    }
}

pub struct Scale {
    target_id: Entity,
    scale: Vec3,
}

impl Scale {
    pub fn new(target_id: Entity, scale: Vec3) -> Self {
        Self { target_id, scale }
    }

    pub fn from_transform(target_id: Entity, transform: &Transform) -> Self {
        Self {
            target_id,
            scale: transform.scale,
        }
    }

    pub fn scale(&mut self, scale: Vec3) -> Action<Transform, Vec3, EmptyRes> {
        let new_scale: Vec3 = self.scale * scale;

        let action: Action<Transform, Vec3, EmptyRes> =
            Action::new(self.target_id, self.scale, new_scale, Self::interp);

        self.scale = new_scale;

        action
    }

    pub fn scale_all(&mut self, scale: f32) -> Action<Transform, Vec3, EmptyRes> {
        let new_scale: Vec3 = self.scale * scale;

        let action: Action<Transform, Vec3, EmptyRes> =
            Action::new(self.target_id, self.scale, new_scale, Self::interp);

        self.scale = new_scale;

        action
    }

    pub fn scale_to(&mut self, scale: Vec3) -> Action<Transform, Vec3, EmptyRes> {
        let action: Action<Transform, Vec3, EmptyRes> =
            Action::new(self.target_id, self.scale, scale, Self::interp);

        self.scale = scale;

        action
    }

    pub fn scale_all_to(&mut self, scale: f32) -> Action<Transform, Vec3, EmptyRes> {
        let new_scale: Vec3 = Vec3::ONE * scale;

        let action: Action<Transform, Vec3, EmptyRes> =
            Action::new(self.target_id, self.scale, new_scale, Self::interp);

        self.scale = new_scale;

        action
    }

    fn interp(
        transform: &mut Transform,
        begin: &Vec3,
        end: &Vec3,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        transform.scale = Vec3::lerp(*begin, *end, t);
    }
}

pub struct Rotation {
    target_id: Entity,
    rotation: Quat,
}

impl Rotation {
    pub fn new(target_id: Entity, rotation: Quat) -> Self {
        Self {
            target_id,
            rotation,
        }
    }

    pub fn from_transform(target_id: Entity, transform: &Transform) -> Self {
        Self {
            target_id,
            rotation: transform.rotation,
        }
    }

    pub fn rotate(&mut self, rotation: Quat) -> Action<Transform, Quat, EmptyRes> {
        let new_rotation: Quat = self.rotation * rotation;

        let action: Action<Transform, Quat, EmptyRes> =
            Action::new(self.target_id, self.rotation, new_rotation, Self::interp);

        self.rotation = new_rotation;

        action
    }

    pub fn rotate_to(&mut self, rotation: Quat) -> Action<Transform, Quat, EmptyRes> {
        let action: Action<Transform, Quat, EmptyRes> =
            Action::new(self.target_id, self.rotation, rotation, Self::interp);

        self.rotation = rotation;

        action
    }

    fn interp(
        transform: &mut Transform,
        begin: &Quat,
        end: &Quat,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        transform.rotation = Quat::slerp(*begin, *end, t);
    }
}
