pub use motiongfx_vello_macros::{VelloBuilder, VelloVector};

use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_vello_renderer::{
    prelude::*,
    vello::{self, kurbo},
};

use crate::{fill_style::FillStyle, stroke_style::StrokeStyle};

pub mod bezpath;
pub mod circle;
pub mod line;
pub mod rect;

pub(crate) trait VelloVector {
    fn shape(&self) -> impl kurbo::Shape;

    #[inline]
    fn build_fill(&self, fill: &FillStyle, scene: &mut vello::Scene) {
        fill.build(scene, &self.shape());
    }

    #[inline]
    fn build_stroke(&self, stroke: &StrokeStyle, scene: &mut vello::Scene) {
        stroke.build(scene, &self.shape());
    }
}

pub(crate) trait VelloBuilder {
    fn is_built(&self) -> bool;

    fn set_built(&mut self, built: bool);
}

pub(crate) fn vector_builder_system<Vector: VelloVector + VelloBuilder + Component>(
    mut q_fill_only_vectors: Query<
        (&mut Vector, &mut FillStyle, &Handle<VelloScene>),
        Without<StrokeStyle>,
    >,
    mut q_stroke_only_vectors: Query<
        (&mut Vector, &mut StrokeStyle, &Handle<VelloScene>),
        Without<FillStyle>,
    >,
    mut q_fill_and_stroke_vectors: Query<(
        &mut Vector,
        &mut FillStyle,
        &mut StrokeStyle,
        &Handle<VelloScene>,
    )>,
    mut scenes: ResMut<Assets<VelloScene>>,
) {
    for (mut vector, mut fill, scene_handle) in q_fill_only_vectors.iter_mut() {
        if let Some(vello_scene) = scenes.get_mut(scene_handle.id()) {
            if vector.is_built() && fill.is_built() {
                continue;
            }

            let mut scene = vello::Scene::new();

            // Build the vector to the VelloScene
            vector.build_fill(&fill, &mut scene);

            // Set it to false after building
            fill.set_built(true);
            vector.set_built(true);

            // Replace with new scene
            vello_scene.scene = scene.into();
        }
    }

    for (mut vector, mut stroke, scene_handle) in q_stroke_only_vectors.iter_mut() {
        if let Some(vello_scene) = scenes.get_mut(scene_handle.id()) {
            if vector.is_built() && stroke.is_built() {
                continue;
            }

            let mut scene = vello::Scene::new();

            // Build the vector to the VelloScene
            vector.build_stroke(&stroke, &mut scene);

            // Set it to false after building
            stroke.set_built(true);
            vector.set_built(true);

            // Replace with new scene
            vello_scene.scene = scene.into();
        }
    }

    for (mut vector, mut fill, mut stroke, scene_handle) in q_fill_and_stroke_vectors.iter_mut() {
        if let Some(vello_scene) = scenes.get_mut(scene_handle.id()) {
            if vector.is_built() && fill.is_built() && stroke.is_built() {
                continue;
            }

            let mut scene = vello::Scene::new();

            // Build the vector to the VelloScene
            vector.build_fill(&fill, &mut scene);
            vector.build_stroke(&stroke, &mut scene);

            // Set it to false after building
            fill.set_built(true);
            stroke.set_built(true);
            vector.set_built(true);

            // Replace with new scene
            vello_scene.scene = scene.into();
        }
    }
}
