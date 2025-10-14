use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

pub mod action;
pub mod color_palette;
pub mod cross_lerp;
pub mod ease;
pub mod lerp;
pub mod sequence;
pub mod timeline;

pub mod prelude {
    pub use crate::{
        action::{Action, ActionBuilder, ActionMeta, ActionMetaGroup},
        color_palette::{ColorKey, ColorPalette},
        cross_lerp::*,
        ease,
        lerp::*,
        sequence::{all, any, chain, delay, flow, Sequence},
        timeline::Timeline,
        EmptyComp, EmptyRes, MotionGfx,
    };
}

pub struct MotionGfx;

impl Plugin for MotionGfx {
    fn build(&self, app: &mut App) {
        app.insert_resource(timeline::Timeline::default())
            .insert_resource(sequence::Sequence::default())
            .insert_resource(EmptyRes)
            .add_systems(PreUpdate, timeline::timeline_update_system);
    }
}

#[derive(Resource)]
pub struct EmptyRes;

#[derive(Component)]
pub struct EmptyComp;
