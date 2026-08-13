//! [`Inspect`] impls for glam's float, signed, and unsigned vector
//! types.
//!
//! Left to the generic struct walk, a `Vec3` would fold its `x`/`y`/`z`
//! away behind a header of its own - technically correct, and not how
//! any engine's inspector shows a vector. This puts the axes on one
//! row instead, each behind a small tinted letter the way Unity,
//! Unreal, and Godot all label them.

use bevy::feathers::controls::{NumberFormat, NumberInputValue};
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;

use fynix_mock::elem;

use super::primitive::number_field;
use super::{Field, Inspect};
use crate::fynix::{Frame, Label};
use crate::palette;
use crate::reactive::BevyUi;
use crate::theme::EditorTheme;

/// Narrower than a lone scalar's, so up to four still sit on one row.
const AXIS_WIDTH: Val = Val::Px(52.0);

/// Which colour an axis takes on, matching the gizmo it moves: none
/// of the engines above agree on much else, but they all tint X red,
/// Y green, Z blue.
fn axis_color(theme: &EditorTheme, name: &str) -> Color {
    match name {
        "x" => palette::RED,
        "y" => palette::GREEN,
        "z" => palette::BLUE,
        _ => theme.text_muted,
    }
}

/// `field`'s child for one axis. `field` itself may be the root value
/// (an empty path), which is the one case `join` would get wrong by
/// leading with a stray dot.
fn axis_field(field: &Field, name: &str) -> Field {
    let path = if field.path().is_empty() {
        name.to_string()
    } else {
        format!("{}.{name}", field.path())
    };
    Field::new(field.target(), path)
}

/// A vector's axes as one row of tight number inputs, each labelled
/// by a single tinted letter rather than the full field name the
/// generic struct walk would have given it.
///
/// `T`/`V`/`format` follow [`number_field`]'s own split between a
/// component's reflected type and the payload its input format
/// emits - a `UVec3` axis rides an `i64` input the same way a lone
/// `u32` does.
fn axes<T, V>(
    field: &Field,
    ui: &mut BevyUi,
    names: &'static [&'static str],
    format: NumberFormat,
    to_field: fn(V) -> T,
    to_input: fn(T) -> NumberInputValue,
) where
    T: FromReflect + PartialReflect,
    V: Clone + Send + Sync + 'static,
    ValueChange<V>: EntityEvent,
{
    let theme = ui.world.resource::<EditorTheme>().clone();
    let field = field.clone();

    ui.elem(elem!(
        Frame,
        direction = FlexDirection::Row,
        align = AlignItems::Center,
        column_gap = px(6)
    ))
    .with(move |ui| {
        for name in names {
            ui.elem(elem!(
                Label,
                text = name.to_uppercase(),
                color = Some(axis_color(&theme, name)),
                bold = true
            ));
            number_field::<T, V>(
                &axis_field(&field, name),
                ui,
                format,
                AXIS_WIDTH,
                to_field,
                to_input,
            );
        }
    });
}

/// Implements [`Inspect`] for one vector family's 2/3/4-component
/// types, given the component type they share and the conversions to
/// and from the input format that edits it - see [`number_field`].
macro_rules! vector_family {
    (
        $comp:ty => $format:ident, $value:ident, $payload:ty,
        $to_field:expr, $to_input:expr;
        $vec2:ty, $vec3:ty, $vec4:ty
    ) => {
        vector_family!(@impl $vec2, &["x", "y"], $comp, $format, $value, $payload, $to_field, $to_input);
        vector_family!(@impl $vec3, &["x", "y", "z"], $comp, $format, $value, $payload, $to_field, $to_input);
        vector_family!(@impl $vec4, &["x", "y", "z", "w"], $comp, $format, $value, $payload, $to_field, $to_input);
    };
    (@impl $ty:ty, $names:expr, $comp:ty, $format:ident, $value:ident, $payload:ty, $to_field:expr, $to_input:expr) => {
        impl Inspect for $ty {
            fn build(field: &Field, ui: &mut BevyUi) {
                axes::<$comp, $payload>(
                    field,
                    ui,
                    $names,
                    NumberFormat::$format,
                    $to_field,
                    |value| NumberInputValue::$value($to_input(value)),
                );
            }
        }
    };
}

vector_family! {
    f32 => F32, F32, f32, |value| value, |value| value;
    Vec2, Vec3, Vec4
}
vector_family! {
    i32 => I32, I32, i32, |value| value, |value| value;
    IVec2, IVec3, IVec4
}
// There is no unsigned input format, so these ride an `i64` and clamp
// on the way back - `as` alone would wrap or truncate.
vector_family! {
    u32 => I64, I64, i64,
    |value: i64| value.clamp(0, u32::MAX as i64) as u32,
    |value| value as i64;
    UVec2, UVec3, UVec4
}
