//! Reflection-driven inspector.
//!
//! [`inspector_fields`] walks any reflected value in the world and
//! renders it as a collapsible hierarchy of editable rows. Which
//! widget a leaf gets is a type-registry lookup, not a match on
//! concrete types, so a new editable type is one [`Inspect`] impl
//! away.
//!
//! A widget is handed a [`Source`] rather than a value, and never
//! learns where that value actually lives. [`Field`] is the one the
//! walk uses - a component of an entity - but anything else the
//! editor keeps can serve the same widgets.

mod enums;
mod field;
mod primitive;
mod tree;
mod vector;

use bevy::prelude::*;
use bevy::reflect::{FromType, GetTypeRegistration, PartialReflect};

use crate::reactive::BevyUi;
pub use field::Field;
pub use tree::{inspector_fields, section};

/// The widgets the inspector can edit out of the box.
///
/// Anything else is one [`InspectAppExt::register_inspect`] away, and
/// needs no change here.
pub struct InspectPlugin;

impl Plugin for InspectPlugin {
    fn build(&self, app: &mut App) {
        app.register_inspect::<bool>()
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
            .register_inspect::<Quat>();
    }
}

/// Registering inspector widgets on the app.
pub trait InspectAppExt {
    /// Makes `T` editable wherever the inspector meets it.
    fn register_inspect<T: Inspect>(&mut self) -> &mut Self;
}

impl InspectAppExt for App {
    fn register_inspect<T: Inspect>(&mut self) -> &mut Self {
        self.register_type::<T>()
            .register_type_data::<T, ReflectInspect>()
    }
}

/// Where a widget reads and writes the value it edits.
///
/// Reflected rather than typed, so it can be handed to a widget the
/// registry picked - which is the only way a caller with a value of
/// unknown type can get the right one.
pub trait Source: Send + Sync + 'static {
    fn get(&self, world: &World) -> Option<Box<dyn PartialReflect>>;

    fn set(&self, world: &mut World, value: &dyn PartialReflect);

    /// Fires when the value may have moved, and on the first poll.
    /// Each source picks its own cheapest signal.
    fn changed(&self)
    -> Box<dyn FnMut(&World) -> bool + Send + Sync>;

    /// A copy of its own, for a widget that needs one per input.
    fn boxed(&self) -> Box<dyn Source>;
}

/// Reading and writing a source as a concrete type, which is what a
/// widget actually wants.
pub trait SourceExt: Source {
    fn read<T: FromReflect>(&self, world: &World) -> Option<T> {
        T::from_reflect(&*self.get(world)?)
    }

    fn write<T: PartialReflect>(&self, world: &mut World, value: T) {
        self.set(world, &value);
    }
}

impl<S: Source + ?Sized> SourceExt for S {}

/// A source's signal, in the shape the kernel polls with. Nothing
/// about a source depends on the node asking.
pub fn when_changed(
    source: &dyn Source,
) -> impl FnMut(&World, Entity) -> bool + Send + Sync + 'static {
    let mut changed = source.changed();
    move |world, _| changed(world)
}

/// The widget for whatever `source` currently holds.
///
/// A registered [`Inspect`] wins; failing that an enum picks its own
/// variant, which needs no registration because reflection already
/// knows what the variants are.
pub fn inspect_value(ui: &mut BevyUi, source: &dyn Source) {
    let Some(value) = source.get(ui.world) else {
        return;
    };

    let drawer = value
        .get_represented_type_info()
        .map(|info| info.type_id())
        .and_then(|type_id| {
            let registry =
                ui.world.resource::<AppTypeRegistry>().read();
            registry.get_type_data::<ReflectInspect>(type_id).cloned()
        });

    if let Some(drawer) = drawer {
        drawer.build(source, ui);
    } else if let Some(variants) = enums::variants(&*value) {
        let pick = enums::all_unit(&*value);
        enums::picker(ui, source, variants, pick);
    }
}

/// Builds the editing widget for one reflected value.
///
/// The value itself is not passed in. A widget is built once and then
/// binds to its source, re-reading whenever that fires - which is what
/// keeps a focused input alive across edits.
pub trait Inspect:
    FromReflect + TypePath + GetTypeRegistration
{
    fn build(source: &dyn Source, ui: &mut BevyUi);
}

/// Type data pointing at a type's [`Inspect::build`].
///
/// A bare `fn`: the source arrives as an argument, so nothing about
/// the widget has to be boxed to be stored.
#[derive(Clone)]
pub struct ReflectInspect {
    build: fn(&dyn Source, &mut BevyUi),
}

impl ReflectInspect {
    pub fn build(&self, source: &dyn Source, ui: &mut BevyUi) {
        (self.build)(source, ui)
    }
}

impl<T: Inspect> FromType<T> for ReflectInspect {
    fn from_type() -> Self {
        Self { build: T::build }
    }
}
