//! [`Backend`]: the concrete [`SceneBackend`] for Bevy, plus a
//! [`default_scene_registry`] wiring `Transform`'s `translation`,
//! `rotation`, and `scale` fields.

use alloc::boxed::Box;

use bevy_math::{Quat, Vec3};
use bevy_transform::components::Transform;
use motiongfx::action::Action;
use motiongfx::ease;
use motiongfx_scene::prelude::*;
use motiongfx_scene::registry::SceneRegistry;
use serde::{Deserialize, Serialize};

use crate::scene::id::SceneId;
use crate::scene::value_pool::{ValueId, ValuePool};
use crate::world::BevyWorld;

/// The concrete [`SceneBackend`] for [`bevy`].
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

/// A [`SceneRegistry`] with `Transform`'s `translation`/`rotation`/
/// `scale` fields, a `To` op for `f32`/`Vec3`/`Quat`, linear
/// interpolation for each, and a couple of named eases.
pub fn default_scene_registry() -> SceneRegistry<Backend> {
    let mut registry = SceneRegistry::new();

    registry.register_field::<Transform, Vec3>(
        "Transform".into(),
        "translation",
        motiongfx::field_path::field_accessor!(
            <Transform>::translation
        ),
    );
    registry.register_field::<Transform, Quat>(
        "Transform".into(),
        "rotation",
        motiongfx::field_path::field_accessor!(<Transform>::rotation),
    );
    registry.register_field::<Transform, Vec3>(
        "Transform".into(),
        "scale",
        motiongfx::field_path::field_accessor!(<Transform>::scale),
    );

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

fn register_to_op<T>(registry: &mut SceneRegistry<Backend>)
where
    T: Clone + Send + Sync + 'static,
{
    registry.register_op::<T, _>(AnimOp::To, |value: &T| {
        let value = value.clone();
        Box::new(move |_prev: &T| value.clone()) as Box<dyn Action<T>>
    });
}
