#![no_std]

extern crate alloc;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
#[cfg(feature = "transform")]
use bevy_transform::TransformSystem;

use crate::accessor::AccessorRegistry;
use crate::field::UntypedField;
use crate::pipeline::PipelineRegistry;

pub mod accessor;
pub mod action;
pub mod ease;
pub mod field;
pub mod interpolation;
pub mod pipeline;
pub mod sequence;
pub mod timeline;
pub mod track;

pub mod prelude {
    pub use crate::action::*;
    pub use crate::ease;
    pub use crate::field::*;
    pub use crate::interpolation::Interpolation;
    pub use crate::sequence::*;
    pub use crate::timeline::*;
}

pub struct MotionGfxEnginePlugin;

impl Plugin for MotionGfxEnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AccessorRegistry<UntypedField>>()
            .init_resource::<PipelineRegistry>();

        app.configure_sets(
            PostUpdate,
            (
                MotionGfxSet::TargetTime,
                MotionGfxSet::MarkAction,
                #[cfg(not(feature = "transform"))]
                MotionGfxSet::Sample,
                #[cfg(feature = "transform")]
                MotionGfxSet::Sample
                    .before(TransformSystem::TransformPropagate),
                MotionGfxSet::CurrentTime,
            )
                .chain(),
        );
    }
}

pub trait FieldPathRegisterAppExt {
    fn register_field_path(&mut self);
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
