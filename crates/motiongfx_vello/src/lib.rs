pub use bevy_vello_renderer;

use bevy::{
    ecs::system::{EntityCommand, EntityCommands},
    math::DVec2,
    prelude::*,
};
use bevy_vello_renderer::{prelude::*, vello::kurbo};
use motiongfx_core::{sequence::update_sequence, UpdateSequenceSet};
use vello_vector::{
    bezpath::VelloBezPath, build_vector, circle::VelloCircle, line::VelloLine, rect::VelloRect,
    Brush, Fill, Stroke,
};

pub mod builder;
pub mod svg;
pub mod vello_vector;

pub mod prelude {
    pub use crate::{
        builder::build_vector,
        vello_vector::{
            bezpath::VelloBezPath, circle::VelloCircle, line::VelloLine, rect::VelloRect, Brush,
            Fill, Stroke,
        },
        AddVelloHandleCommandExtension, MotionGfxVelloPlugin,
    };

    pub use bevy_vello_renderer::prelude::*;
}

pub struct MotionGfxVelloPlugin;

impl Plugin for MotionGfxVelloPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(VelloRenderPlugin).add_systems(
            Update,
            (
                // Vector builders
                build_vector::<VelloRect>(),
                build_vector::<VelloCircle>(),
                build_vector::<VelloLine>(),
                build_vector::<VelloBezPath>(),
                // Sequences
                update_sequence::<Fill, Brush>,
                update_sequence::<Stroke, Brush>,
                update_sequence::<Stroke, kurbo::Stroke>,
                update_sequence::<Stroke, f64>,
                // VelloCircle
                update_sequence::<VelloCircle, VelloCircle>,
                update_sequence::<VelloCircle, f64>,
                // VelloRect
                update_sequence::<VelloRect, VelloRect>,
                update_sequence::<VelloRect, DVec2>,
                update_sequence::<VelloRect, f64>,
                // VelloLine
                update_sequence::<VelloLine, VelloLine>,
                update_sequence::<VelloLine, DVec2>,
                update_sequence::<VelloLine, f64>,
                // VelloBezPath
                update_sequence::<VelloBezPath, f32>,
            )
                .in_set(UpdateSequenceSet),
        );
    }
}

pub trait AddVelloHandleCommandExtension {
    fn add_vello_handle(&mut self) -> &mut Self;
}

impl<'a> AddVelloHandleCommandExtension for EntityCommands<'a> {
    fn add_vello_handle(&mut self) -> &mut Self {
        self.add(AddVelloHandleCommand);
        self
    }
}

pub struct AddVelloHandleCommand;

impl EntityCommand for AddVelloHandleCommand {
    fn apply(self, id: Entity, world: &mut World) {
        let mut vello_scenes = world
            .get_resource_mut::<Assets<VelloScene>>()
            .expect("Assets<VelloScene> resource not initialized. MotionGfxVelloPlugin is needed.");

        let vello_handle = vello_scenes.add(VelloScene::default());

        world.entity_mut(id).insert(vello_handle);

        // SpatialBundle is needed for Vello graphics to be visible to the camera
        let transform = world.entity(id).get::<Transform>().copied();
        let visibility = world.entity(id).get::<Visibility>().copied();

        world.entity_mut(id).insert(SpatialBundle {
            transform: transform.unwrap_or_default(),
            visibility: visibility.unwrap_or_default(),
            ..default()
        });
    }
}
