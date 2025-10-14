use bevy::prelude::*;
use sequence::{sequence_controller, sequence_player};
use slide::slide_controller;

pub mod action;
pub mod color_palette;
pub mod ease;
pub mod f32lerp;
pub mod sequence;
pub mod slide;
pub mod tuple_motion;

pub mod prelude {
    pub use crate::{
        action::{act, Action, SequenceBuilderExt},
        color_palette::{ColorKey, ColorPalette},
        ease,
        f32lerp::F32Lerp,
        sequence::{
            all, any, chain, delay, flow, update_asset, update_component, MultiSeqOrd, Sequence,
            SequenceBundle, SequenceController, SequencePlayer, SequencePlayerBundle, SingleSeqOrd,
        },
        slide::{create_slide, SlideBundle, SlideController, SlideCurrState, SlideTargetState},
        tuple_motion::{GetId, GetMut, GetMutValue},
    };
}

pub struct MotionGfxCorePlugin;

impl Plugin for MotionGfxCorePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (sequence_player, slide_controller).before(UpdateSequenceSet),
        )
        .add_systems(Update, sequence_controller.after(UpdateSequenceSet));
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UpdateSequenceSet;
