use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_vello_renderer::prelude::*;

pub mod rect;

pub(crate) trait VelloVector {
    fn build(&self, fragment: &mut VelloFragment);

    fn should_build(&self) -> bool;

    fn set_should_build(&mut self, should_build: bool);
}

pub(crate) fn vector_builder<Vector: VelloVector + Component>(
    mut q_vectors: Query<(&mut Vector, &Handle<VelloFragment>)>,
    mut fragments: ResMut<Assets<VelloFragment>>,
) {
    for (mut vector, fragment_handle) in q_vectors.iter_mut() {
        if let Some(fragment) = fragments.get_mut(fragment_handle.id()) {
            if vector.should_build() {
                // Build the vector to the VelloFragment
                vector.build(fragment);

                // Set it to false after building
                vector.set_should_build(false);
            }
        }
    }
}
