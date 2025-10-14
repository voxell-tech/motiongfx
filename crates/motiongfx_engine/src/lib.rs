use bevy::prelude::*;

// For docs.
#[allow(unused_imports)]
use crate::sequence::SequenceController;

pub mod action;
pub mod animate;
pub mod ease;
pub mod field;
pub mod interpolation;
pub mod sequence;
pub mod timeline;

pub mod prelude {
    pub use crate::action::*;
    pub use crate::animate::AnimateAppExt;
    pub use crate::field::*;
    pub use crate::interpolation::Interpolation;
    pub use crate::sequence::*;
    pub use crate::timeline::*;
    pub use crate::{ease, MotionGfxSet};
}

pub struct MotionGfxEnginePlugin;

impl Plugin for MotionGfxEnginePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            field::FieldPlugin,
            sequence::SequencePlugin,
            timeline::TimelinePlugin,
        ));

        app.configure_sets(
            PostUpdate,
            (
                MotionGfxSet::TargetTime,
                MotionGfxSet::MarkAction,
                MotionGfxSet::Sample
                    .before(TransformSystem::TransformPropagate),
                MotionGfxSet::CurrentTime,
            )
                .chain(),
        );
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MotionGfxSet {
    /// Advance the target time in the [`SequenceController`].
    TargetTime,
    /// Mark actions that are affected by the `target_time`
    /// change in [`SequenceController`].
    MarkAction,
    /// Sample keyframes and applies the value.
    /// This happens before [`TransformSystem::TransformPropagate`].
    Sample,
    /// Advance the current time in the [`SequenceController`].
    CurrentTime,
}

/// Auto trait for types that implements [`Send`] + [`Sync`] + `'static`.
pub trait ThreadSafe: Send + Sync + 'static {}

impl<T> ThreadSafe for T where T: Send + Sync + 'static {}
