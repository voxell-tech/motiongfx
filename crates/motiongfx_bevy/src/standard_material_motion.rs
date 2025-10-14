use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_pbr::prelude::*;
use bevy_render::prelude::*;
use motiongfx_core::prelude::*;

pub struct StandardMaterialMotion {
    target_id: Entity,
    material: StandardMaterial,
}

impl StandardMaterialMotion {
    pub fn new(target_id: Entity, material: StandardMaterial) -> Self {
        Self {
            target_id,
            material,
        }
    }

    pub fn base_color_to(
        &mut self,
        color: Color,
    ) -> Action<Handle<StandardMaterial>, Color, Assets<StandardMaterial>> {
        let action: Action<Handle<StandardMaterial>, Color, Assets<StandardMaterial>> = Action::new(
            self.target_id,
            self.material.base_color,
            color,
            Self::base_color_interp,
        );

        self.material.base_color = color;

        action
    }

    pub fn base_alpha_to(
        &mut self,
        alpha: f32,
    ) -> Action<Handle<StandardMaterial>, Color, Assets<StandardMaterial>> {
        let mut new_color: Color = self.material.base_color;
        new_color.set_a(alpha);

        let action: Action<Handle<StandardMaterial>, Color, Assets<StandardMaterial>> = Action::new(
            self.target_id,
            self.material.base_color,
            new_color,
            Self::base_color_interp,
        );

        self.material.base_color = new_color;

        action
    }

    fn base_color_interp(
        material_handle: &mut Handle<StandardMaterial>,
        begin: &Color,
        end: &Color,
        t: f32,
        materials: &mut ResMut<Assets<StandardMaterial>>,
    ) {
        if let Some(material) = materials.get_mut(material_handle.id()) {
            material.base_color = Color::lerp(begin, end, t);
        }
    }

    pub fn emissive_to(
        &mut self,
        color: Color,
    ) -> Action<Handle<StandardMaterial>, Color, Assets<StandardMaterial>> {
        let action: Action<Handle<StandardMaterial>, Color, Assets<StandardMaterial>> = Action::new(
            self.target_id,
            self.material.emissive,
            color,
            Self::emissive_interp,
        );

        self.material.emissive = color;

        action
    }

    fn emissive_interp(
        material_handle: &mut Handle<StandardMaterial>,
        begin: &Color,
        end: &Color,
        t: f32,
        materials: &mut ResMut<Assets<StandardMaterial>>,
    ) {
        if let Some(material) = materials.get_mut(material_handle.id()) {
            material.emissive = Color::lerp(begin, end, t);
        }
    }
}
