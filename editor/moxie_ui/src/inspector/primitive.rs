//! [`Inspect`] impls for the primitive types the inspector edits out
//! of the box.

use bevy::feathers::controls::{NumberFormat, NumberInputValue};
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;

use bevy_fynix::WorldEntityMut;
use fynix::WorldNodeRef;
use fynix::elem;

use crate::elements::{
    CheckBox, CheckBoxCursor, NumberField, NumberFieldCursor,
};
use crate::reactive::BevyUi;

use super::{Inspect, Source, SourceExt, when_changed};

/// A checkbox. `Checked` is a marker, inserted or removed rather than
/// written, so this binds raw instead of writing a component.
impl Inspect for bool {
    fn build(source: &dyn Source, ui: &mut BevyUi) {
        let edited = source.boxed();
        let read = source.boxed();
        let checked = read.read::<bool>(ui.world).unwrap_or_default();

        ui.elem(elem!(CheckBox, checked = checked))
            .observe(
                move |change: On<ValueChange<bool>>,
                      mut commands: Commands| {
                    let (source, value) =
                        (edited.boxed(), change.value);

                    commands.queue(move |world: &mut World| {
                        source.write(world, value);
                    });
                },
            )
            // Controlled: what it shows follows the value it edits,
            // and only moves once the write has landed.
            .bind(
                |b| b.checked(),
                when_changed(source),
                move |WorldNodeRef { world, .. }| {
                    read.read::<bool>(world).unwrap_or_default()
                },
            );
    }
}

/// A number input for a numeric leaf.
///
/// `V` is the payload `number_field` emits, which follows
/// `format` rather than the field's own type - a `u32` is edited
/// through an `i64` input and converted on the way in and out.
/// `width` is exposed rather than left at the widget's own default so
/// a vector's per-axis fields can sit narrower than a lone scalar.
pub(super) fn number_field<T, V>(
    source: &dyn Source,
    ui: &mut BevyUi,
    format: NumberFormat,
    to_value: fn(V) -> T,
    to_input: fn(T) -> NumberInputValue,
) where
    T: FromReflect,
    V: Clone + Send + Sync + 'static,
    ValueChange<V>: EntityEvent,
{
    let edited = source.boxed();
    let read = source.boxed();

    let shown = read.read::<T>(ui.world).map(to_input);

    ui.elem(elem!(
        NumberField,
        format = format,
        value = shown.unwrap_or(NumberInputValue::F32(0.0))
    ))
    .observe(
        move |change: On<ValueChange<V>>, mut commands: Commands| {
            let (source, value) =
                (edited.boxed(), change.value.clone());

            commands.queue(move |world: &mut World| {
                source.write(world, to_value(value));
            });
        },
    )
    .bind(
        |input| input.value(),
        when_changed(source),
        move |WorldNodeRef { world, .. }| {
            read.read::<T>(world)
                .map(to_input)
                .unwrap_or(NumberInputValue::F32(0.0))
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
            fn build(source: &dyn Source, ui: &mut BevyUi) {
                number_field::<$ty, $payload>(
                    source,
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
    // clamp on the way back - `as` alone would wrap or truncate.
    u32 => I64, I64, i64, |value: i64| value.clamp(0, u32::MAX as i64) as u32, |value| value as i64;
    u64 => I64, I64, i64, |value: i64| value.max(0) as u64, |value| value as i64;
}
