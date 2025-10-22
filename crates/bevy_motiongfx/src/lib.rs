//! [Bevy]: https://bevyengine.org/
//! [MotionGfx]: motiongfx
//!
//! An integration of the [MotionGfx] crate for the [Bevy] game engine.
//!
//! ## Getting Started
//!
//! To start animating a cube from the left to right:
//!
//! ```
//! use bevy::prelude::*;
//! use bevy_motiongfx::BevyMotionGfxPlugin;
//! use bevy_motiongfx::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins((DefaultPlugins, BevyMotionGfxPlugin))
//!         .add_systems(Startup, (setup, build_timeline))
//!         .run();
//! }
//!
//! /// Spawns the camera and the directional light.
//! fn setup(mut commands: Commands) {
//!     commands.spawn((
//!         Camera3d::default(),
//!         Transform::from_xyz(0.0, 0.0, 15.0),
//!     ));
//!
//!     commands.spawn((
//!         DirectionalLight::default(),
//!         Transform::from_xyz(3.0, 10.0, 5.0)
//!             .looking_at(Vec3::ZERO, Vec3::Y),
//!     ));
//! }
//!
//! /// Creates the timeline and plays it.
//! fn build_timeline(
//!     mut commands: Commands,
//!     mut meshes: ResMut<Assets<Mesh>>,
//!     mut materials: ResMut<Assets<StandardMaterial>>,
//! ) {
//!     // Spawns the cube.
//!     let cube = commands
//!         .spawn((
//!             Mesh3d(meshes.add(Cuboid::default())),
//!             MeshMaterial3d(
//!                 materials.add(StandardMaterial::default()),
//!             ),
//!             Transform::from_xyz(-3.0, 0.0, 0.0),
//!         ))
//!         .id();
//!
//!     // Build the timeline.
//!     let mut b = TimelineBuilder::new();
//!     let track = b
//!         .act_interp(cube, field!(<Transform>::translation::x), |x| {
//!             x + 6.0
//!         })
//!         .play(1.0)
//!         .compile();
//!
//!     b.add_tracks(track);
//!     let timeline = b.compile();
//!
//!     // Spawns the timeline and start playing.
//!     commands
//!         .spawn((timeline, RealtimePlayer::new().with_playing(true)));
//! }
//! ```

#![no_std]

use bevy_app::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use motiongfx::field_path::accessor::FieldAccessorRegistry as AccessorRegistry;

use crate::controller::ControllerPlugin;
use crate::pipeline::{PipelinePlugin, WorldPipelineRegistry};

pub mod controller;
pub mod interpolation;
pub mod pipeline;
pub mod registry;

pub mod prelude {
    pub use motiongfx::prelude::*;

    pub use crate::FieldAccessorRegistry;
    pub use crate::controller::RealtimePlayer;
    pub use crate::interpolation::{
        ActionInterpTimelineExt, Interpolation,
    };
    pub use crate::pipeline::{
        PipelineRegistryExt, WorldPipeline, WorldPipelineRegistry,
    };
    pub use crate::register_fields;
    pub use crate::registry::FieldPathRegisterAppExt;
}

pub use motiongfx;

#[derive(Resource, Default, Debug, Deref, DerefMut)]
pub struct FieldAccessorRegistry(AccessorRegistry);

pub struct BevyMotionGfxPlugin;

impl Plugin for BevyMotionGfxPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            (
                MotionGfxSet::Controller,
                MotionGfxSet::Bake,
                MotionGfxSet::QueueAction,
                #[cfg(not(feature = "transform"))]
                MotionGfxSet::Sample,
                #[cfg(feature = "transform")]
                MotionGfxSet::Sample.before(
                    bevy_transform::TransformSystems::Propagate,
                ),
            )
                .chain(),
        );

        app.init_resource::<FieldAccessorRegistry>()
            .init_resource::<WorldPipelineRegistry>();

        app.add_plugins((PipelinePlugin, ControllerPlugin));

        #[cfg(feature = "transform")]
        {
            use bevy_transform::components::Transform;

            register_fields!(
                app.register_component_field(),
                Transform,
                (
                    translation(x, y, z),
                    scale(x, y, z),
                    rotation(x, y, z, w),
                )
            );
        }

        #[cfg(feature = "sprite")]
        {
            use bevy_sprite::prelude::*;

            register_fields!(
                app.register_component_field(),
                Sprite,
                (
                    image,
                    texture_atlas,
                    color,
                    flip_x,
                    flip_y,
                    custom_size,
                    rect,
                    image_mode,
                )
            );
        }

        #[cfg(feature = "pbr")]
        {
            use bevy_pbr::prelude::*;

            register_fields!(
                app.register_asset_field(),
                StandardMaterial,
                (
                    base_color,
                    emissive,
                    perceptual_roughness,
                    metallic,
                    reflectance,
                    specular_tint,
                    diffuse_transmission,
                    specular_transmission,
                    thickness,
                    ior,
                    attenuation_distance,
                    attenuation_color,
                )
            );
        }
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MotionGfxSet {
    /// [Controller](controller) update to the timeline.
    Controller,
    /// Bake actions into segments.
    Bake,
    /// Queue actions that will be sampled by marking them.
    QueueAction,
    /// Sample keyframes and applies the value.
    Sample,
}
