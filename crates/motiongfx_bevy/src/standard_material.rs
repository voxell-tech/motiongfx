use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_pbr::prelude::*;
use motiongfx_core::prelude::*;

type StdMatAsset = Assets<StandardMaterial>;
type StdMatHandle = Handle<StandardMaterial>;

pub struct BaseColor {
    target_id: Entity,
    color: Vec4,
}

impl BaseColor {
    pub fn new(target_id: Entity, color: Vec4) -> Self {
        Self { target_id, color }
    }

    pub fn from_material(target_id: Entity, material: &StandardMaterial) -> Self {
        Self {
            target_id,
            color: material.base_color.into(),
        }
    }

    pub fn multiply(&mut self, color: Vec4) -> Action<StdMatHandle, Vec4, StdMatAsset> {
        let new_color: Vec4 = self.color * color;

        let action: Action<StdMatHandle, Vec4, StdMatAsset> =
            Action::new(self.target_id, self.color, new_color, Self::interp);

        self.color = new_color;

        action
    }

    pub fn add(&mut self, color: Vec4) -> Action<StdMatHandle, Vec4, StdMatAsset> {
        let new_color: Vec4 = self.color + color;

        let action: Action<StdMatHandle, Vec4, StdMatAsset> =
            Action::new(self.target_id, self.color, new_color, Self::interp);

        self.color = new_color;

        action
    }

    pub fn color_to(&mut self, color: Vec4) -> Action<StdMatHandle, Vec4, StdMatAsset> {
        let action: Action<StdMatHandle, Vec4, StdMatAsset> =
            Action::new(self.target_id, self.color, color, Self::interp);

        self.color = color;

        action
    }

    pub fn alpha_to(&mut self, alpha: f32) -> Action<StdMatHandle, Vec4, StdMatAsset> {
        let mut new_color: Vec4 = self.color;
        new_color.w = alpha;

        let action: Action<StdMatHandle, Vec4, StdMatAsset> =
            Action::new(self.target_id, self.color, new_color, Self::interp);

        self.color = new_color;

        action
    }

    fn interp(
        material_handle: &mut StdMatHandle,
        begin: &Vec4,
        end: &Vec4,
        t: f32,
        materials: &mut ResMut<StdMatAsset>,
    ) {
        if let Some(material) = materials.get_mut(material_handle.id()) {
            material.base_color = Vec4::lerp(*begin, *end, t).into();
        }
    }
}

pub struct Emissive {
    target_id: Entity,
    color: Vec4,
}

impl Emissive {
    pub fn new(target_id: Entity, color: Vec4) -> Self {
        Self { target_id, color }
    }

    pub fn from_material(target_id: Entity, material: &StandardMaterial) -> Self {
        Self {
            target_id,
            color: material.emissive.into(),
        }
    }

    pub fn multiply(&mut self, color: Vec4) -> Action<StdMatHandle, Vec4, StdMatAsset> {
        let new_color: Vec4 = self.color * color;

        let action: Action<StdMatHandle, Vec4, StdMatAsset> =
            Action::new(self.target_id, self.color, new_color, Self::interp);

        self.color = new_color;

        action
    }

    pub fn add(&mut self, color: Vec4) -> Action<StdMatHandle, Vec4, StdMatAsset> {
        let new_color: Vec4 = self.color + color;

        let action: Action<StdMatHandle, Vec4, StdMatAsset> =
            Action::new(self.target_id, self.color, new_color, Self::interp);

        self.color = new_color;

        action
    }

    pub fn color_to(&mut self, color: Vec4) -> Action<StdMatHandle, Vec4, StdMatAsset> {
        let action: Action<StdMatHandle, Vec4, StdMatAsset> =
            Action::new(self.target_id, self.color, color, Self::interp);

        self.color = color;

        action
    }

    fn interp(
        material_handle: &mut StdMatHandle,
        begin: &Vec4,
        end: &Vec4,
        t: f32,
        materials: &mut ResMut<StdMatAsset>,
    ) {
        if let Some(material) = materials.get_mut(material_handle.id()) {
            material.emissive = Vec4::lerp(*begin, *end, t).into();
        }
    }
}
