use bevy::prelude::*;
use slide::slide_controller;

// For docs.
#[allow(unused_imports)]
use crate::sequence::SequenceController;

pub mod action;
pub mod ease;
pub mod field;
pub mod interpolation;
pub mod sequence;
pub mod slide;

pub mod prelude {
    pub use crate::action::*;
    pub use crate::field::*;
    pub use crate::interpolation::Interpolation;
    pub use crate::sequence::*;
    pub use crate::slide::{
        create_slide, SlideBundle, SlideController, SlideCurrState,
        SlideTargetState,
    };
    pub use crate::{ease, MotionGfxSet};
}

pub struct MotionGfxEnginePlugin;

impl Plugin for MotionGfxEnginePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            sequence::SequencePlugin,
            field::FieldPlugin,
        ));

        app.configure_sets(
            PostUpdate,
            (
                MotionGfxSet::TargetTime,
                MotionGfxSet::MarkTrack,
                MotionGfxSet::Sample
                    .before(TransformSystem::TransformPropagate),
                MotionGfxSet::CurrentTime,
            )
                .chain(),
        );
        app.add_systems(
            PostUpdate,
            ((slide_controller).in_set(MotionGfxSet::TargetTime),),
        );
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MotionGfxSet {
    /// Advance the target time in the [`SequenceController`].
    TargetTime,
    /// Mark tracks that is affected by the target time
    /// change in [`SequenceController`].
    MarkTrack,
    /// Sample keyframes and applies the value.
    /// This happens before [`TransformSystem::TransformPropagate`].
    Sample,
    /// Advance the current time in the [`SequenceController`].
    CurrentTime,
}

/// Auto trait for types that implements [`Send`] + [`Sync`] + `'static`.
pub trait ThreadSafe: Send + Sync + 'static {}

impl<T> ThreadSafe for T where T: Send + Sync + 'static {}
