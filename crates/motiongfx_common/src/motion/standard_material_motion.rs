use bevy::prelude::*;
use motiongfx_core::prelude::*;

pub trait StandardMaterialMotion<const N: usize> {
    fn std_material(&mut self) -> StandardMaterialMotionBuilder;
}

impl<const N: usize, T: GetMutValue<StandardMaterial, N>>
    StandardMaterialMotion<N> for (Entity, T)
{
    fn std_material(&mut self) -> StandardMaterialMotionBuilder {
        StandardMaterialMotionBuilder::new(
            self.id(),
            self.1.get_mut_value(),
        )
    }
}

pub struct StandardMaterialMotionBuilder<'a> {
    pub id: Entity,
    pub material: &'a mut StandardMaterial,
}

impl<'a> StandardMaterialMotionBuilder<'a> {
    pub fn new(
        id: Entity,
        material: &'a mut StandardMaterial,
    ) -> Self {
        Self { id, material }
    }

    pub fn to_emissive(
        &mut self,
        color: LinearRgba,
    ) -> Action<LinearRgba, StandardMaterial> {
        act!(
            (self.id, StandardMaterial),
            start = { self.material }.emissive,
            end = color,
        )
    }

    pub fn to_base_color(
        &mut self,
        color: Color,
    ) -> Action<Color, StandardMaterial> {
        act!(
            (self.id, StandardMaterial),
            start = { self.material }.base_color,
            end = color,
        )
    }
}
