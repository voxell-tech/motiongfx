use bevy::prelude::*;
use bevy_vello_graphics::prelude::*;
use motiongfx_core::prelude::*;

pub trait StrokeMotion<const N: usize> {
    fn stroke(&mut self) -> StrokeMotionBuilder;
}

impl<const N: usize, T: GetMutValue<Stroke, N>> StrokeMotion<N> for (Entity, T) {
    fn stroke(&mut self) -> StrokeMotionBuilder {
        StrokeMotionBuilder::new(self.id(), self.1.get_mut_value())
    }
}

pub struct StrokeMotionBuilder<'a> {
    id: Entity,
    pub stroke: &'a mut Stroke,
}

impl<'a> StrokeMotionBuilder<'a> {
    pub fn new(id: Entity, stroke: &'a mut Stroke) -> Self {
        Self { id, stroke }
    }

    pub fn to_width(&mut self, width: f64) -> Action<f64, Stroke> {
        act!(
            (self.id, Stroke),
            start = { self.stroke }.style.width,
            end = width,
        )
    }
}
