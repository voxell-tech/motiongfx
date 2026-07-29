#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use crate::controller::ControllerPlugin;
use crate::manager::MotionGfxManagerPlugin;
#[cfg(feature = "scene")]
use crate::scene::MotionGfxScenePlugin;

pub mod controller;
pub mod interpolation;
pub mod manager;
pub mod world;

#[cfg(feature = "scene")]
pub mod scene;

pub mod prelude {
    pub use motiongfx::prelude::*;

    pub use crate::controller::{FixedRatePlayer, RealtimePlayer};
    pub use crate::manager::{MotionGfxManager, TimelineId};
    pub use crate::world::{BevyTimeline, BevyTimelineBuilder};

    #[cfg(feature = "scene")]
    pub use crate::scene::id::SceneId;
}

pub use motiongfx;

pub struct BevyMotionGfxPlugin;

impl Plugin for BevyMotionGfxPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            (
                MotionGfxSystems::Controller,
                #[cfg(not(feature = "transform"))]
                MotionGfxSystems::Sample,
                #[cfg(feature = "transform")]
                MotionGfxSystems::Sample.before(
                    bevy_transform::TransformSystems::Propagate,
                ),
            )
                .chain(),
        );
        app.add_plugins((MotionGfxManagerPlugin, ControllerPlugin));

        #[cfg(feature = "scene")]
        app.add_plugins(MotionGfxScenePlugin);
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MotionGfxSystems {
    /// [Controller](controller) update to the timeline.
    Controller,
    /// Sample keyframes and applies the value.
    Sample,
}
