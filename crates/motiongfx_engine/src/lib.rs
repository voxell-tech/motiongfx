#![no_std]

extern crate alloc;

use bevy_app::prelude::*;
use bevy_ecs::component::Mutable;
use bevy_ecs::prelude::*;
#[cfg(feature = "transform")]
use bevy_transform::TransformSystem;

use crate::accessor::{Accessor, FieldAccessorRegistry};
use crate::field::Field;
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
        app.init_resource::<FieldAccessorRegistry>()
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
    fn register_component_field<S, T>(
        &mut self,
        field: Field<S, T>,
        accesor: Accessor<S, T>,
    ) where
        S: Component<Mutability = Mutable>,
        T: Clone + ThreadSafe;
}

impl FieldPathRegisterAppExt for App {
    fn register_component_field<S, T>(
        &mut self,
        field: Field<S, T>,
        accesor: Accessor<S, T>,
    ) where
        S: Component<Mutability = Mutable>,
        T: Clone + ThreadSafe,
    {
        self.world_mut()
            .resource_mut::<FieldAccessorRegistry>()
            .register(field.untyped(), accesor);

        let pipeline_key = self
            .world_mut()
            .resource_mut::<PipelineRegistry>()
            .register_component::<S, T>();
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
