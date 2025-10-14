use bevy::prelude::*;
use bevy_vello_graphics::{bevy_vello::vello::peniko, prelude::*};
use motiongfx_core::prelude::*;

pub trait FillMotion<const N: usize> {
    fn fill(&mut self) -> FillMotionBuilder;
}

impl<const N: usize, T: GetMutValue<Fill, N>> FillMotion<N> for (Entity, T) {
    fn fill(&mut self) -> FillMotionBuilder {
        FillMotionBuilder::new(self.id(), self.1.get_mut_value())
    }
}

pub struct FillMotionBuilder<'a> {
    id: Entity,
    pub fill: &'a mut Fill,
}

impl<'a> FillMotionBuilder<'a> {
    pub fn new(id: Entity, fill: &'a mut Fill) -> Self {
        Self { id, fill }
    }

    pub fn to_color(&mut self, color: Color) -> Action<peniko::Brush, Fill> {
        let linear = color.to_linear();
        act!(
            (self.id, Fill),
            start = { self.fill }.brush.value,
            end = peniko::Brush::Solid(peniko::Color::rgba(
                linear.red as f64,
                linear.green as f64,
                linear.blue as f64,
                linear.alpha as f64
            )),
        )
    }
}
