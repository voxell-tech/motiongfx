use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

pub mod action;
pub mod color_palette;
pub mod cross_lerp;
pub mod ease;
pub mod lerp;
pub mod sequence;
pub mod slide;

pub mod prelude {
    pub use crate::{
        action::{Action, ActionBuilder},
        color_palette::{ColorKey, ColorPalette},
        cross_lerp::*,
        ease,
        lerp::*,
        sequence::{
            all, any, chain, delay, flow, Sequence, SequenceBundle, SequenceController,
            SequencePlayer, SequencePlayerBundle,
        },
        slide::{create_slide, SlideBundle, SlideController, SlideCurrState, SlideTargetState},
        EmptyComp, EmptyRes, MotionGfx,
    };
}

pub struct MotionGfx;

impl Plugin for MotionGfx {
    fn build(&self, app: &mut App) {
        app.insert_resource(EmptyRes)
            .add_systems(PreUpdate, sequence::sequence_controller_system)
            .add_systems(
                Update,
                (
                    sequence::sequence_player_system,
                    slide::slide_controller_system,
                ),
            );
    }
}

#[derive(Resource)]
pub struct EmptyRes;

#[derive(Component)]
pub struct EmptyComp;

/// Calculate if 2 time range (in float) overlaps.
pub(crate) fn time_range_overlap(a_begin: f32, a_end: f32, b_begin: f32, b_end: f32) -> bool {
    a_begin <= b_end && b_begin <= a_end
}
