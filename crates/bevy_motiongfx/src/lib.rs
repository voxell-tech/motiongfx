#![doc = include_str!("../README.md")]
#![no_std]

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use crate::controller::ControllerPlugin;
use crate::world::MotionGfxWorldPlugin;

pub mod controller;
pub mod interpolation;
pub mod pipeline;
pub mod world;

pub mod prelude {
    pub use motiongfx::prelude::*;

    pub use crate::controller::{FixedRatePlayer, RealtimePlayer};
    pub use crate::interpolation::{
        ActionInterpTimelineExt, Interpolation,
    };
    pub use crate::pipeline::{BevyTimeline, BevyTimelineBuilder};
    pub use crate::world::{MotionGfxWorld, TimelineId};
}

pub use motiongfx;

pub struct BevyMotionGfxPlugin;

impl Plugin for BevyMotionGfxPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            (
                MotionGfxSet::Controller,
                #[cfg(not(feature = "transform"))]
                MotionGfxSet::Sample,
                #[cfg(feature = "transform")]
                MotionGfxSet::Sample.before(
                    bevy_transform::TransformSystems::Propagate,
                ),
            )
                .chain(),
        );
        app.add_plugins((MotionGfxWorldPlugin, ControllerPlugin));
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MotionGfxSet {
    /// [Controller](controller) update to the timeline.
    Controller,
    /// Sample keyframes and applies the value.
    Sample,
}
