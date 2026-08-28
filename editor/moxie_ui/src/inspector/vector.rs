//! [`Inspect`] impls for glam's float, signed, and unsigned vector
//! types.
//!
//! Left to the generic struct walk, a `Vec3` would fold its `x`/`y`/`z`
//! away behind a header of its own - technically correct, and not how
//! any engine's inspector shows a vector. This puts the axes on one
//! row instead, each behind a small tinted letter the way Unity,
//! Unreal, and Godot all label them.
//!
//! Each input edits the whole vector: it reads one out, replaces a
//! component, and writes it back. So the widget needs no way to
//! address an axis on its own, and serves any [`Source`] - a
//! component's field or a value the editor keeps elsewhere.

use bevy::feathers::controls::{NumberFormat, NumberInputValue};
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;

use bevy_fynix::EntityExt;
use fynix_mock::elem;
use fynix_mock::WorldNodeRef;

use super::{Inspect, Source, SourceExt, when_changed};
use crate::elements::{Frame, Label, NumberField, NumberFieldCursor};
use crate::monokai;
use crate::reactive::BevyUi;
use crate::theme::EditorTheme;

/// A vector, by the axes an inspector edits it through.
trait Axes: FromReflect + Send + Sync + 'static {
    /// What one axis holds.
    type Axis;
    /// One per axis, in order.
    const NAMES: &'static [&'static str];

    fn axis(&self, index: usize) -> Self::Axis;
    fn set_axis(&mut self, index: usize, value: Self::Axis);
}

/// Which colour an axis takes on, matching the gizmo it moves: none
/// of the engines above agree on much else, but they all tint X red,
/// Y green, Z blue.
fn axis_color(theme: &EditorTheme, name: &str) -> Color {
    match name {
        "x" => monokai::RED,
        "y" => monokai::GREEN,
        "z" => monokai::BLUE,
        _ => theme.text_muted,
    }
}

/// A vector's axes as one row of tight number inputs, each labelled
/// by a single tinted letter rather than the full field name the
/// generic struct walk would have given it.
///
/// `V` is the payload the input emits, which follows `format` rather
/// than the axis's own type - an unsigned axis rides an `i64` input
/// and is clamped on the way back.
fn axes<T, V>(
    source: &dyn Source,
    ui: &mut BevyUi,
    format: NumberFormat,
    to_axis: fn(V) -> T::Axis,
    to_input: fn(T::Axis) -> NumberInputValue,
) where
    T: Axes,
    V: Clone + Send + Sync + 'static,
    ValueChange<V>: EntityEvent,
{
    let source = source.boxed();

    ui.elem(elem!(
        Frame,
        direction = FlexDirection::Row,
        align = AlignItems::Center,
        column_gap = px(6)
    ))
    .with(move |ui| {
        for (index, name) in T::NAMES.iter().enumerate() {
            ui.elem(elem!(
                Label,
                text = name.to_uppercase(),
                color = Some(axis_color(ui.theme, name)),
                bold = true
            ));
            axis::<T, V>(
                &*source, ui, index, format, to_axis, to_input,
            );
        }
    });
}

/// One axis, as a number input over the whole vector.
fn axis<T, V>(
    source: &dyn Source,
    ui: &mut BevyUi,
    index: usize,
    format: NumberFormat,
    to_axis: fn(V) -> T::Axis,
    to_input: fn(T::Axis) -> NumberInputValue,
) where
    T: Axes,
    V: Clone + Send + Sync + 'static,
    ValueChange<V>: EntityEvent,
{
    let edited = source.boxed();
    let read = source.boxed();
    let initial = shown::<T>(source, ui.world, index, to_input);

    ui.elem(elem!(
        NumberField,
        format = format,
        width = px(40),
        value = initial
    ))
    .observe(
        move |change: On<ValueChange<V>>, mut commands: Commands| {
            let (source, value) =
                (edited.boxed(), change.value.clone());

            commands.queue(move |world: &mut World| {
                // Read, replace, write back: the source addresses the
                // vector, never one axis of it.
                let Some(mut vector) = source.read::<T>(world) else {
                    return;
                };
                vector.set_axis(index, to_axis(value));
                source.write(world, vector);
            });
        },
    )
    .bind(
        |input| input.value(),
        when_changed(source),
        move |WorldNodeRef { world, .. }| shown::<T>(&*read, world, index, to_input),
    );
}

/// What the input for `index` should be showing.
fn shown<T: Axes>(
    source: &dyn Source,
    world: &World,
    index: usize,
    to_input: fn(T::Axis) -> NumberInputValue,
) -> NumberInputValue {
    source
        .read::<T>(world)
        .map(|vector| to_input(vector.axis(index)))
        .unwrap_or(NumberInputValue::F32(0.0))
}

/// One vector type, by the axes it is edited through and the
/// conversions to and from the input format that edits them.
macro_rules! vector {
    (
        $ty:ty,
        axes = [$($name:literal),*],
        axis = $axis:ty,
        format = $format:ident,
        value = $value:ident,
        payload = $payload:ty,
        $to_axis:expr,
        $to_input:expr
    ) => {
        impl Axes for $ty {
            type Axis = $axis;
            const NAMES: &'static [&'static str] = &[$($name),*];

            fn axis(&self, index: usize) -> $axis {
                self.to_array()[index]
            }

            fn set_axis(&mut self, index: usize, value: $axis) {
                let mut axes = self.to_array();
                axes[index] = value;
                *self = <$ty>::from_array(axes);
            }
        }

        impl Inspect for $ty {
            fn build(source: &dyn Source, ui: &mut BevyUi) {
                axes::<$ty, $payload>(
                    source,
                    ui,
                    NumberFormat::$format,
                    $to_axis,
                    |value| NumberInputValue::$value($to_input(value)),
                );
            }
        }
    };
}

/// A family's 2/3/4-component types, which differ only in how many
/// axes they carry.
macro_rules! vector_family {
    (
        axis = $axis:ty,
        format = $format:ident,
        value = $value:ident,
        payload = $payload:ty,
        $to_axis:expr,
        $to_input:expr,
        [$vec2:ty, $vec3:ty, $vec4:ty]
    ) => {
        vector!(
            $vec2,
            axes = ["x", "y"],
            axis = $axis,
            format = $format,
            value = $value,
            payload = $payload,
            $to_axis,
            $to_input
        );
        vector!(
            $vec3,
            axes = ["x", "y", "z"],
            axis = $axis,
            format = $format,
            value = $value,
            payload = $payload,
            $to_axis,
            $to_input
        );
        vector!(
            $vec4,
            axes = ["x", "y", "z", "w"],
            axis = $axis,
            format = $format,
            value = $value,
            payload = $payload,
            $to_axis,
            $to_input
        );
    };
}

vector_family!(
    axis = f32,
    format = F32,
    value = F32,
    payload = f32,
    |value| value,
    |value| value,
    [Vec2, Vec3, Vec4]
);
vector_family!(
    axis = i32,
    format = I32,
    value = I32,
    payload = i32,
    |value| value,
    |value| value,
    [IVec2, IVec3, IVec4]
);
// There is no unsigned input format, so these ride an `i64` and clamp
// on the way back - `as` alone would wrap or truncate.
vector_family!(
    axis = u32,
    format = I64,
    value = I64,
    payload = i64,
    |value: i64| value.clamp(0, u32::MAX as i64) as u32,
    |value| value as i64,
    [UVec2, UVec3, UVec4]
);

// A rotation is four axes like any other, and one tinted row reads
// far better than the folded group of floats the walk would give it.
vector!(
    Quat,
    axes = ["x", "y", "z", "w"],
    axis = f32,
    format = F32,
    value = F32,
    payload = f32,
    |value| value,
    |value| value
);
