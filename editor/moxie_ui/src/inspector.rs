//! Reflection-driven inspector.
//!
//! [`inspector_fields`] walks any reflected value in the world and
//! renders it as a collapsible hierarchy of editable rows. Which
//! widget a leaf gets is a type-registry lookup, not a match on
//! concrete types, so a new editable type is one [`Inspect`] impl
//! away.
//!
//! A [`Field`] says where the value lives: one component of one
//! entity, and the reflect path to a leaf inside it. Bevy keeps a
//! resource on an entity of its own, so a resource is a component
//! too, and nothing here needs to know which it was handed.

mod field;
mod primitive;
mod tree;
mod vector;

use bevy::prelude::*;
use bevy::reflect::{FromType, GetTypeRegistration};

use crate::reactive::BevyUi;
pub use field::Field;
pub use tree::{inspector_fields, section};

/// Builds the editing widget for one reflected value.
///
/// The value itself is not passed in. A widget is built once and then
/// binds to `field`, re-reading through it whenever the target
/// changes, which is what keeps a focused input alive across edits.
pub trait Inspect: Reflect + TypePath + GetTypeRegistration {
    fn build(field: &Field, ui: &mut BevyUi);
}

/// Type data pointing at a type's [`Inspect::build`].
///
/// The function needs no downcast, unlike a drawer that takes the
/// value: `build` is resolved from the type it was registered for, and
/// the widget reads its own value back through [`Field`].
#[derive(Clone)]
pub struct ReflectInspect {
    build: fn(&Field, &mut BevyUi),
}

impl ReflectInspect {
    pub fn build(&self, field: &Field, ui: &mut BevyUi) {
        (self.build)(field, ui)
    }
}

impl<T: Inspect> FromType<T> for ReflectInspect {
    fn from_type() -> Self {
        Self { build: T::build }
    }
}

/// Registering inspector widgets on the app.
pub trait InspectAppExt {
    /// Makes `T` editable wherever the inspector meets it.
    fn register_inspect<T: Inspect>(&mut self) -> &mut Self;

    /// Registers the primitives the inspector can edit out of the box.
    fn register_default_inspects(&mut self) -> &mut Self;
}

impl InspectAppExt for App {
    fn register_inspect<T: Inspect>(&mut self) -> &mut Self {
        self.register_type::<T>()
            .register_type_data::<T, ReflectInspect>()
    }

    fn register_default_inspects(&mut self) -> &mut Self {
        self.register_inspect::<bool>()
            .register_inspect::<f32>()
            .register_inspect::<f64>()
            .register_inspect::<i32>()
            .register_inspect::<i64>()
            .register_inspect::<u32>()
            .register_inspect::<u64>()
            .register_inspect::<Vec2>()
            .register_inspect::<Vec3>()
            .register_inspect::<Vec4>()
            .register_inspect::<IVec2>()
            .register_inspect::<IVec3>()
            .register_inspect::<IVec4>()
            .register_inspect::<UVec2>()
            .register_inspect::<UVec3>()
            .register_inspect::<UVec4>()
    }
}
