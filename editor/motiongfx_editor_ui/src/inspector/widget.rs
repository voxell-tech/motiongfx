//! How a reflected type presents itself for editing.
//!
//! A type becomes editable by implementing [`Inspect`] and registering
//! it, which stores a [`ReflectInspect`] against the type in bevy's
//! [`TypeRegistry`](bevy::reflect::TypeRegistry). The inspector then
//! finds widgets by lookup, so it never grows a match over concrete
//! types and a downstream crate can add its own.

use bevy::feathers::controls::{
    NumberFormat, NumberInputValue, UpdateNumberInput,
};
use bevy::prelude::*;
use bevy::reflect::{FromType, GetTypeRegistration};
use bevy::ui::Checked;
use bevy::ui_widgets::ValueChange;

use super::{Field, apply_scene_to, target_changed};
use crate::glass::{glass_checkbox, glass_number_field};
use crate::reactive::BevyUi;

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
    }
}

/// A checkbox. `Checked` is a marker, inserted or removed rather than
/// written, so this binds raw instead of writing a component.
impl Inspect for bool {
    fn build(field: &Field, ui: &mut BevyUi) {
        let edited = field.clone();
        let read = field.clone();
        ui.node(move |world, node| {
            apply_scene_to(world, node, glass_checkbox());
            let field = edited.clone();
            world.entity_mut(node).observe(
                move |change: On<ValueChange<bool>>,
                      mut commands: Commands| {
                    let (field, value) =
                        (field.clone(), change.value);
                    commands.queue(move |world: &mut World| {
                        field.set(world, value);
                    });
                    // The checkbox is controlled, so its own state
                    // only moves once the write lands.
                    if value {
                        commands
                            .entity(change.source)
                            .insert(Checked);
                    } else {
                        commands
                            .entity(change.source)
                            .remove::<Checked>();
                    }
                },
            );
        })
        .bind_raw(
            target_changed(field.target()),
            move |world, node| {
                if read.get::<bool>(world).unwrap_or_default() {
                    world.entity_mut(node).insert(Checked);
                } else {
                    world.entity_mut(node).remove::<Checked>();
                }
            },
        );
    }
}

/// A number input for a numeric leaf.
///
/// `V` is the payload [`glass_number_field`] emits, which follows
/// `format` rather than the field's own type - a `u32` is edited
/// through an `i64` input and converted on the way in and out.
fn number_field<T, V>(
    field: &Field,
    ui: &mut BevyUi,
    format: NumberFormat,
    to_field: fn(V) -> T,
    to_input: fn(T) -> NumberInputValue,
) where
    T: FromReflect + PartialReflect,
    V: Clone + Send + Sync + 'static,
    ValueChange<V>: EntityEvent,
{
    let edited = field.clone();
    let read = field.clone();
    ui.node(move |world, node| {
        apply_scene_to(world, node, glass_number_field(format));
        let field = edited.clone();
        world.entity_mut(node).observe(
            move |change: On<ValueChange<V>>,
                  mut commands: Commands| {
                let (field, value) =
                    (field.clone(), change.value.clone());
                commands.queue(move |world: &mut World| {
                    field.set(world, to_field(value));
                });
            },
        );
    })
    .bind_raw(
        target_changed(field.target()),
        move |world, node| {
            let Some(value) = read.get::<T>(world) else {
                return;
            };
            // Pushed as an event rather than a component write: a focused
            // input ignores it, so a live edit still wins.
            world.trigger(UpdateNumberInput {
                entity: node,
                value: to_input(value),
            });
        },
    );
}

/// Implements [`Inspect`] for a numeric type, given the input format
/// it is edited through and the conversions to and from that input's
/// payload.
macro_rules! number_widget {
    ($(
        $ty:ty => $format:ident, $value:ident, $payload:ty,
        $to_field:expr, $to_input:expr;
    )*) => {$(
        impl Inspect for $ty {
            fn build(field: &Field, ui: &mut BevyUi) {
                number_field::<$ty, $payload>(
                    field,
                    ui,
                    NumberFormat::$format,
                    $to_field,
                    |value| NumberInputValue::$value($to_input(value)),
                );
            }
        }
    )*};
}

number_widget! {
    f32 => F32, F32, f32, |value| value, |value| value;
    f64 => F64, F64, f64, |value| value, |value| value;
    i32 => I32, I32, i32, |value| value, |value| value;
    i64 => I64, I64, i64, |value| value, |value| value;
    // There is no unsigned input format, so these ride an `i64` and
    // clamp on the way back - `as` alone would wrap a negative edit
    // around to a huge value.
    u32 => I64, I64, i64, |value: i64| value.max(0) as u32, |value| value as i64;
    u64 => I64, I64, i64, |value: i64| value.max(0) as u64, |value| value as i64;
}
