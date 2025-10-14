use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use prelude::*;

pub mod ease;

mod action;
mod color_palette;
mod cross_lerp;
mod lerp;
mod sequence;
mod timeline;

pub mod prelude {
    pub use crate::{
        action::{Action, ActionBuilder, ActionMeta, ActionMetaGroup, InterpFn},
        color_palette::{ColorKey, ColorPalette},
        cross_lerp::*,
        ease,
        lerp::*,
        sequence::{all, any, chain, delay, flow, sequence_player_system, Sequence},
        timeline::Timeline,
        EmptyComp, EmptyRes, MotionGfx,
    };
}

pub struct MotionGfx;

impl Plugin for MotionGfx {
    fn build(&self, app: &mut App) {
        app.insert_resource(Timeline::default())
            .insert_resource(Sequence::default())
            .insert_resource(EmptyRes)
            .add_systems(PreUpdate, timeline::timeline_update_system);
    }
}

#[derive(Resource)]
pub struct EmptyRes;

#[derive(Component)]
pub struct EmptyComp;
