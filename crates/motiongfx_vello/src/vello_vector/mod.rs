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
    fn shape(&self) -> &impl kurbo::Shape {
        &kurbo::Rect::ZERO
    }

    fn build_fill(&self, fill: &FillStyle, builder: &mut vello::SceneBuilder) {
        fill.build(builder, self.shape());
    }

    fn build_stroke(&self, stroke: &StrokeStyle, builder: &mut vello::SceneBuilder) {
        stroke.build(builder, self.shape());
    }
}

pub(crate) trait VelloBuilder {
    fn is_built(&self) -> bool;

    fn set_built(&mut self, built: bool);
}

pub(crate) fn vector_builder<Vector: VelloVector + VelloBuilder + Component>(
    mut q_fill_only_vectors: Query<
        (&mut Vector, &mut FillStyle, &Handle<VelloFragment>),
        Without<StrokeStyle>,
    >,
    mut q_stroke_only_vectors: Query<
        (&mut Vector, &mut StrokeStyle, &Handle<VelloFragment>),
        Without<FillStyle>,
    >,
    mut q_fill_and_stroke_vectors: Query<(
        &mut Vector,
        &mut FillStyle,
        &mut StrokeStyle,
        &Handle<VelloFragment>,
    )>,
    mut fragments: ResMut<Assets<VelloFragment>>,
) {
    for (mut vector, mut fill, fragment_handle) in q_fill_only_vectors.iter_mut() {
        if let Some(fragment) = fragments.get_mut(fragment_handle.id()) {
            let mut frag: vello::SceneFragment = vello::SceneFragment::new();
            let mut builder: vello::SceneBuilder = vello::SceneBuilder::for_fragment(&mut frag);

            if vector.is_built() && fill.is_built() {
                continue;
            }

            // Build the vector to the VelloFragment
            vector.build_fill(&fill, &mut builder);

            // Set it to false after building
            fill.set_built(true);
            vector.set_built(true);

            // Replace with new fragment
            fragment.fragment = frag.into();
        }
    }

    for (mut vector, mut stroke, fragment_handle) in q_stroke_only_vectors.iter_mut() {
        if let Some(fragment) = fragments.get_mut(fragment_handle.id()) {
            let mut frag: vello::SceneFragment = vello::SceneFragment::new();
            let mut builder: vello::SceneBuilder = vello::SceneBuilder::for_fragment(&mut frag);

            if vector.is_built() && stroke.is_built() {
                continue;
            }

            // Build the vector to the VelloFragment
            vector.build_stroke(&stroke, &mut builder);

            // Set it to false after building
            stroke.set_built(true);
            vector.set_built(true);

            // Replace with new fragment
            fragment.fragment = frag.into();
        }
    }

    for (mut vector, mut fill, mut stroke, fragment_handle) in q_fill_and_stroke_vectors.iter_mut()
    {
        if let Some(fragment) = fragments.get_mut(fragment_handle.id()) {
            let mut frag: vello::SceneFragment = vello::SceneFragment::new();
            let mut builder: vello::SceneBuilder = vello::SceneBuilder::for_fragment(&mut frag);

            if vector.is_built() && fill.is_built() && stroke.is_built() {
                continue;
            }

            // Build the vector to the VelloFragment
            vector.build_fill(&fill, &mut builder);
            vector.build_stroke(&stroke, &mut builder);

            // Set it to false after building
            fill.set_built(true);
            stroke.set_built(true);
            vector.set_built(true);

            // Replace with new fragment
            fragment.fragment = frag.into();
        }
    }
}
