#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use crate::controller::ControllerPlugin;
use crate::manager::MotionGfxManagerPlugin;
#[cfg(feature = "velyst")]
use crate::velyst_integration::VelystIntegrationPlugin;

pub mod controller;
pub mod interpolation;
pub mod manager;
#[cfg(feature = "velyst")]
pub mod velyst_integration;
pub mod world;

pub mod prelude {
    pub use motiongfx::prelude::*;

    pub use crate::controller::{FixedRatePlayer, RealtimePlayer};
    pub use crate::manager::{MotionGfxManager, TimelineId};
    pub use crate::world::{BevyTimeline, BevyTimelineBuilder};

    #[cfg(feature = "velyst")]
    pub use crate::velyst_integration::{
        KanvaAnim, KanvaGroup, KanvaGroupKind, KanvaPhase,
    };
    #[cfg(feature = "velyst")]
    pub use velyst::prelude::*;
}

pub use motiongfx;
#[cfg(feature = "velyst")]
pub use velyst;

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

        #[cfg(feature = "velyst")]
        app.add_plugins(VelystIntegrationPlugin);
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MotionGfxSystems {
    /// [Controller](controller) update to the timeline.
    Controller,
    /// Sample keyframes and applies the value.
    Sample,
}
