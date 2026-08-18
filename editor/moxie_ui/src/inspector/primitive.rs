//! [`Inspect`] impls for the primitive types the inspector edits out
//! of the box.

use bevy::feathers::controls::{NumberFormat, NumberInputValue};
use bevy::prelude::*;
use bevy::text::{EditableText, TextEditChange};
use bevy::ui_widgets::ValueChange;

use bevy_fynix::ElementMutExt;
use fynix_mock::elem;

use crate::elements::{
    CheckBox, CheckBoxCursor, NumberField, NumberFieldCursor,
    TextField, TextFieldCursor,
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
                move |world, _| {
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
    width: Val,
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
        width = width,
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
        move |world, _| {
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
                    px(110),
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

/// A single-line text input.
///
/// `T` is whatever the leaf actually is - `String` reads and writes
/// through as-is, `Name` converts at the edges - the same shape
/// [`number_field`] takes for numeric payloads.
///
/// Feathers has no `ValueChange` for text the way it does for
/// numbers, so this listens for [`TextEditChange`] directly - which
/// also fires on a bare cursor move, hence the guard against writing
/// back a value that has not actually changed.
fn text_field<T: FromReflect>(
    source: &dyn Source,
    ui: &mut BevyUi,
    to_value: fn(String) -> T,
    to_shown: fn(&T) -> String,
) {
    let edited = source.boxed();
    let read = source.boxed();
    let shown = read
        .read::<T>(ui.world)
        .as_ref()
        .map(to_shown)
        .unwrap_or_default();

    let field =
        ui.elem(elem!(TextField, value = shown, width = px(110)));
    let node = field.id();

    field.bind(
        |input| input.value(),
        when_changed(source),
        move |world, _| {
            read.read::<T>(world)
                .as_ref()
                .map(to_shown)
                .unwrap_or_default()
        },
    );

    let Some(children) =
        ui.world.get::<Children>(node).map(|c| c.to_vec())
    else {
        return;
    };
    let Some(&text_input) = children.iter().find(|&&child| {
        ui.world.get::<EditableText>(child).is_some()
    }) else {
        return;
    };

    ui.world.entity_mut(text_input).observe(
        move |change: On<TextEditChange>,
              texts: Query<&EditableText>,
              mut commands: Commands| {
            let Ok(text) = texts.get(change.event_target()) else {
                return;
            };
            let (source, value) =
                (edited.boxed(), text.value().to_string());

            commands.queue(move |world: &mut World| {
                if source
                    .read::<T>(world)
                    .as_ref()
                    .map(to_shown)
                    .as_deref()
                    != Some(value.as_str())
                {
                    source.write(world, to_value(value));
                }
            });
        },
    );
}

impl Inspect for String {
    fn build(source: &dyn Source, ui: &mut BevyUi) {
        text_field(source, ui, |value| value, String::clone);
    }
}

impl Inspect for Name {
    fn build(source: &dyn Source, ui: &mut BevyUi) {
        text_field(source, ui, Name::new, |name| {
            name.as_str().to_string()
        });
    }
}
