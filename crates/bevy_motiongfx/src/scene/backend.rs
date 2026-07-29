//! [`Backend`]: the concrete [`SceneBackend`] for Bevy, plus a
//! [`default_scene_registry`] wiring `Transform`'s `translation`,
//! `rotation`, and `scale` fields.

use alloc::boxed::Box;

use bevy_reflect::TypePath;
use bevy_transform::components::Transform;
use motiongfx::interpolation::Interpolation;
use motiongfx::prelude::*;
use motiongfx_scene::prelude::*;
use motiongfx_scene::registry::SceneRegistry;
use serde::{Deserialize, Serialize};

use crate::scene::id::SceneId;
use crate::scene::value_pool::{ValueId, ValuePool};
use crate::world::BevyWorld;

/// The concrete [`SceneBackend`] for Bevy.
pub struct Backend;

impl SceneBackend for Backend {
    type Id = SceneId;
    type ValueId = ValueId;
    type ValuePool = ValuePool;
    type OpId = AnimOp;
    type InterpId = AnimInterp;
    type EaseId = AnimEase;
    type World = BevyWorld;
}

pub type BackendRegistry = SceneRegistry<Backend>;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum AnimOp {
    /// Sets the field directly to the action's value.
    To,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum AnimInterp {
    Linear,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum AnimEase {
    Linear,
    CubicEaseInOut,
}

pub trait SceneRegistryExt {
    fn register_reflected_field<S, T>(
        &mut self,
        field_acc: FieldAccessor<S, T>,
    ) -> &mut Self
    where
        S: TypePath,
        BevyWorld: SubjectSource<SceneId, S>,
        ValuePool: ValueColumn<ValueId, T>,
        T: ThreadSafe + Clone;

    /// Registers a `To` op for `T`: sets the field directly to the
    /// action's value.
    fn register_to_op<T>(&mut self) -> &mut Self
    where
        T: Clone + Send + Sync + 'static;

    /// Registers linear interpolation for `T`, reusing `T`'s own
    /// [`Interpolation<M>`] impl instead of a hand-written lerp/slerp
    /// closure.
    fn register_linear_interp<T, M>(&mut self) -> &mut Self
    where
        T: Interpolation<M> + 'static;

    /// Registers a field plus its `To` op and linear interpolation, in
    /// one call - just pass `path!(<S>::field)`.
    fn register_bundle<S, T, M>(
        &mut self,
        field_acc: FieldAccessor<S, T>,
    ) -> &mut Self
    where
        S: TypePath,
        BevyWorld: SubjectSource<SceneId, S>,
        ValuePool: ValueColumn<ValueId, T>,
        T: Interpolation<M> + ThreadSafe + Clone,
    {
        self.register_reflected_field(field_acc)
            .register_to_op::<T>()
            .register_linear_interp::<T, M>()
    }
}

impl SceneRegistryExt for BackendRegistry {
    fn register_reflected_field<S, T>(
        &mut self,
        field_acc: FieldAccessor<S, T>,
    ) -> &mut Self
    where
        S: TypePath,
        BevyWorld: SubjectSource<SceneId, S>,
        ValuePool: ValueColumn<ValueId, T>,
        T: ThreadSafe + Clone,
    {
        self.register_field(TypeName::new(S::type_path()), field_acc)
    }

    fn register_to_op<T>(&mut self) -> &mut Self
    where
        T: Clone + Send + Sync + 'static,
    {
        self.register_op::<T, _>(AnimOp::To, |value: &T| {
            let value = value.clone();
            Box::new(move |_prev: &T| value.clone())
                as Box<dyn Action<T>>
        })
    }

    fn register_linear_interp<T, M>(&mut self) -> &mut Self
    where
        T: Interpolation<M> + 'static,
    {
        self.register_interp::<T>(
            AnimInterp::Linear,
            <T as Interpolation<M>>::interp,
        )
    }
}

/// Create a default battery included scene registry!
pub fn default_scene_registry() -> BackendRegistry {
    let mut registry = SceneRegistry::new();

    registry
        .register_bundle(path!(<Transform>::translation))
        .register_bundle(path!(<Transform>::rotation))
        .register_bundle(path!(<Transform>::scale))
        .register_eases(&[
            (AnimEase::Linear, ease::linear),
            (AnimEase::CubicEaseInOut, ease::cubic::ease_in_out),
        ]);

    registry
}
