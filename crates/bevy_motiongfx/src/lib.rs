#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use crate::controller::ControllerPlugin;
use crate::manager::MotionGfxManagerPlugin;

pub mod controller;
pub mod interpolation;
#[cfg(feature = "velyst")]
pub mod kanva;
pub mod manager;
pub mod world;

pub mod prelude {
    pub use motiongfx::prelude::*;

    pub use crate::controller::{FixedRatePlayer, RealtimePlayer};
    pub use crate::manager::{MotionGfxManager, TimelineId};
    pub use crate::world::{BevyTimeline, BevyTimelineBuilder};

    #[cfg(feature = "velyst")]
    pub use crate::kanva::{
        KanvaAnim, KanvaGroup, KanvaGroupKind, KanvaPhase,
    };
    #[cfg(feature = "velyst")]
    pub use velyst::prelude::*;
}

pub use motiongfx;
#[cfg(feature = "velyst")]
pub use velyst;
#[cfg(feature = "velyst")]
pub use crate::kanva::VelystMotionGfxPlugin;

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
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MotionGfxSystems {
    /// [Controller](controller) update to the timeline.
    Controller,
    /// Sample keyframes and applies the value.
    Sample,
}
