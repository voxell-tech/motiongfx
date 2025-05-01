use bevy::asset::AsAssetId;
use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use sequence::{animate_asset, animate_component, update_curr_time, update_target_time};
use slide::slide_controller;

pub mod action;
pub mod color_palette;
pub mod ease;
pub mod f32lerp;
pub mod sequence;
pub mod slide;
pub mod tuple_motion;

pub mod prelude {
    pub use crate::action::{act, Action, SequenceBuilderExt};
    pub use crate::color_palette::{ColorKey, ColorPalette};
    pub use crate::f32lerp::F32Lerp;
    pub use crate::sequence::{
        all, any, chain, delay, flow, MultiSeqOrd, Sequence, SequenceBundle, SequenceController,
        SequencePlayer, SequencePlayerBundle, SingleSeqOrd,
    };
    pub use crate::slide::{
        create_slide, SlideBundle, SlideController, SlideCurrState, SlideTargetState,
    };
    pub use crate::tuple_motion::{GetId, GetMut, GetMutValue};
    pub use crate::{ease, MotionGfxAnimateAppExt, MotionGfxSet};
}

pub struct MotionGfxCorePlugin;

impl Plugin for MotionGfxCorePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            (MotionGfxSet::Time, MotionGfxSet::Animate).chain(),
        );

        app.add_systems(
            Update,
            ((update_target_time, slide_controller), update_curr_time)
                .chain()
                .in_set(MotionGfxSet::Time),
        );
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MotionGfxSet {
    Time,
    Animate,
}

/// Utility function for registering animatable components.
pub trait MotionGfxAnimateAppExt {
    fn animate_component<Comp, Target>(&mut self) -> &mut Self
    where
        Comp: Component<Mutability = Mutable>,
        Target: ThreadSafe;

    fn animate_asset<Comp, Target>(&mut self) -> &mut Self
    where
        Comp: Component + AsAssetId,
        Target: ThreadSafe;
}

impl MotionGfxAnimateAppExt for App {
    fn animate_component<Comp, Target>(&mut self) -> &mut Self
    where
        Comp: Component<Mutability = Mutable>,
        Target: ThreadSafe,
    {
        self.add_systems(
            PostUpdate,
            animate_component::<Comp, Target>.in_set(MotionGfxSet::Animate),
        )
    }

    fn animate_asset<Comp, Target>(&mut self) -> &mut Self
    where
        Comp: Component + AsAssetId,
        Target: ThreadSafe,
    {
        self.add_systems(
            PostUpdate,
            animate_asset::<Comp, Target>.in_set(MotionGfxSet::Animate),
        )
    }
}

/// Auto trait for types that implements [`Send`] + [`Sync`] + `'static`.
pub trait ThreadSafe: Send + Sync + 'static {}

impl<T> ThreadSafe for T where T: Send + Sync + 'static {}
