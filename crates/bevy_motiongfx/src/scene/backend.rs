//! [`Backend`]: the concrete [`SceneBackend`] for Bevy, plus a
//! [`default_scene_registry`] wiring `Transform`'s `translation`,
//! `rotation`, and `scale` fields.

use alloc::boxed::Box;

use bevy_math::{Quat, Vec3};
use bevy_reflect::TypePath;
use bevy_transform::components::Transform;
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

pub trait SceneRegistryFieldExt<S, T> {
    fn register_reflected_field(
        &mut self,
        field_acc: FieldAccessor<S, T>,
    );
}

impl<S, T> SceneRegistryFieldExt<S, T> for BackendRegistry
where
    S: TypePath,
    BevyWorld: SubjectSource<SceneId, S>,
    ValuePool: ValueColumn<ValueId, T>,
    T: ThreadSafe + Clone,
{
    fn register_reflected_field(
        &mut self,
        field_acc: FieldAccessor<S, T>,
    ) {
        self.register_field(TypeName::new(S::type_path()), field_acc);
    }
}

/// A [`SceneRegistry`] with `Transform`'s `translation`/`rotation`/
/// `scale` fields, a `To` op for `f32`/`Vec3`/`Quat`, linear
/// interpolation for each, and a couple of named eases.
pub fn default_scene_registry() -> BackendRegistry {
    let mut registry = SceneRegistry::new();

    registry
        .register_reflected_field(path!(<Transform>::translation));
    registry.register_reflected_field(path!(<Transform>::rotation));
    registry.register_reflected_field(path!(<Transform>::scale));

    register_to_op::<f32>(&mut registry);
    register_to_op::<Vec3>(&mut registry);
    register_to_op::<Quat>(&mut registry);

    registry.register_interp::<f32>(AnimInterp::Linear, |a, b, t| {
        a + (b - a) * t
    });
    registry
        .register_interp::<Vec3>(AnimInterp::Linear, |a, b, t| {
            a.lerp(*b, t)
        });
    registry
        .register_interp::<Quat>(AnimInterp::Linear, |a, b, t| {
            a.slerp(*b, t)
        });

    registry.register_ease(AnimEase::Linear, ease::linear);
    registry.register_ease(
        AnimEase::CubicEaseInOut,
        ease::cubic::ease_in_out,
    );

    registry
}

fn register_to_op<T>(registry: &mut BackendRegistry)
where
    T: Clone + Send + Sync + 'static,
{
    registry.register_op::<T, _>(AnimOp::To, |value: &T| {
        let value = value.clone();
        Box::new(move |_prev: &T| value.clone()) as Box<dyn Action<T>>
    });
}
