use bevy::asset::AsAssetId;
use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use sequence::{
    animate_asset, animate_component, update_target_time, update_time,
};
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
        all, any, chain, delay, flow, MultiSeqOrd, Sequence,
        SequenceBundle, SequenceController, SequencePlayer,
        SequencePlayerBundle, SingleSeqOrd,
    };
    pub use crate::slide::{
        create_slide, SlideBundle, SlideController, SlideCurrState,
        SlideTargetState,
    };
    pub use crate::tuple_motion::{GetId, GetMut, GetMutValue};
    pub use crate::{ease, MotionGfxAnimateAppExt, MotionGfxSet};
}

pub struct MotionGfxCorePlugin;

impl Plugin for MotionGfxCorePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            (
                MotionGfxSet::TargetTime,
                MotionGfxSet::Animate,
                MotionGfxSet::Time,
            )
                .chain(),
        );

        app.add_systems(
            PostUpdate,
            (
                (update_target_time, slide_controller)
                    .in_set(MotionGfxSet::TargetTime),
                update_time.in_set(MotionGfxSet::Time),
            ),
        );
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MotionGfxSet {
    /// Advance the target time in the [`sequence::SequenceController`].
    TargetTime,
    /// Perform animation updates.
    Animate,
    /// Advance the time in the [`sequence::SequenceController`].
    Time,
}

/// Utility function for registering animatable components.
pub trait MotionGfxAnimateAppExt {
    fn animate_component<Comp, Field>(&mut self) -> &mut Self
    where
        Comp: Component<Mutability = Mutable>,
        Field: ThreadSafe;

    fn animate_asset<Comp, Target>(&mut self) -> &mut Self
    where
        Comp: Component + AsAssetId,
        Target: ThreadSafe;
}

impl MotionGfxAnimateAppExt for App {
    fn animate_component<Comp, Field>(&mut self) -> &mut Self
    where
        Comp: Component<Mutability = Mutable>,
        Field: ThreadSafe,
    {
        self.add_systems(
            PostUpdate,
            animate_component::<Comp, Field>
                .in_set(MotionGfxSet::Animate),
        )
    }

    fn animate_asset<Comp, Field>(&mut self) -> &mut Self
    where
        Comp: Component + AsAssetId,
        Field: ThreadSafe,
    {
        self.add_systems(
            PostUpdate,
            animate_asset::<Comp, Field>
                .in_set(MotionGfxSet::Animate),
        )
    }
}

/// Auto trait for types that implements [`Send`] + [`Sync`] + `'static`.
pub trait ThreadSafe: Send + Sync + 'static {}

impl<T> ThreadSafe for T where T: Send + Sync + 'static {}
