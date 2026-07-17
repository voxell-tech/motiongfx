//! Generic reflection-driven inspector.
//!
//! [`ReflectInspectorPlugin<T>`] renders a `Reflect` resource's
//! fields as editable rows under any entity carrying
//! [`Inspector<T>`]: `bool` becomes a feathers checkbox, numbers
//! become number inputs. Edits are written back through reflect
//! paths, and external changes to the resource are synced back into
//! the widgets.

use std::marker::PhantomData;

use bevy::ecs::component::Mutable;
use bevy::feathers::controls::{
    FeathersCheckbox, FeathersNumberInput, NumberFormat,
    NumberInputValue, UpdateNumberInput,
};
use bevy::feathers::theme::ThemedText;
use bevy::prelude::*;
use bevy::reflect::{GetPath, ReflectRef};
use bevy::ui::Checked;
use bevy::ui_widgets::ValueChange;

use crate::ui::glass;

/// Registers the build / edit / sync systems for one resource type.
pub struct ReflectInspectorPlugin<T>(PhantomData<T>);

impl<T> Default for ReflectInspectorPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Resource<Mutability = Mutable> + Reflect> Plugin
    for ReflectInspectorPlugin<T>
{
    fn build(&self, app: &mut App) {
        app.add_observer(build_inspector::<T>)
            .add_observer(on_change_bool::<T>)
            .add_observer(on_change_number::<T, f32>)
            .add_observer(on_change_number::<T, f64>)
            .add_observer(on_change_number::<T, i32>)
            .add_observer(on_change_number::<T, i64>)
            .add_systems(Update, sync_inspector::<T>);
    }
}

/// Marker: build editable rows for resource `T` under this entity.
#[derive(Component)]
pub struct Inspector<T: Resource + Reflect>(PhantomData<T>);

impl<T: Resource + Reflect> Default for Inspector<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// A widget bound to one reflect path (e.g. `physical_size.x`) of
/// `T`.
#[derive(Component)]
pub struct InspectorField<T: Resource + Reflect> {
    pub path: String,
    _marker: PhantomData<T>,
}

impl<T: Resource + Reflect> InspectorField<T> {
    fn new(path: String) -> Self {
        Self {
            path,
            _marker: PhantomData,
        }
    }
}

/// A leaf (editable) field resolved from the reflect tree.
enum Leaf {
    Bool(bool),
    Number(NumberFormat, NumberInputValue),
}

/// Flatten nested structs into `(path, leaf)` rows; unsupported kinds
/// are skipped.
fn collect_leaves(
    value: &dyn PartialReflect,
    prefix: &str,
    out: &mut Vec<(String, Leaf)>,
) {
    if let Some(leaf) = as_leaf(value) {
        out.push((prefix.to_string(), leaf));
        return;
    }
    if let ReflectRef::Struct(s) = value.reflect_ref() {
        for i in 0..s.field_len() {
            let Some(name) = s.name_at(i) else { continue };
            let Some(field) = s.field_at(i) else { continue };
            let path = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{prefix}.{name}")
            };
            collect_leaves(field, &path, out);
        }
    }
}

fn as_leaf(value: &dyn PartialReflect) -> Option<Leaf> {
    use NumberFormat as F;
    use NumberInputValue as V;
    let v = value;
    if let Some(b) = v.try_downcast_ref::<bool>() {
        Some(Leaf::Bool(*b))
    } else if let Some(x) = v.try_downcast_ref::<f32>() {
        Some(Leaf::Number(F::F32, V::F32(*x)))
    } else if let Some(x) = v.try_downcast_ref::<f64>() {
        Some(Leaf::Number(F::F64, V::F64(*x)))
    } else if let Some(x) = v.try_downcast_ref::<i32>() {
        Some(Leaf::Number(F::I32, V::I32(*x)))
    } else if let Some(x) = v.try_downcast_ref::<i64>() {
        Some(Leaf::Number(F::I64, V::I64(*x)))
    } else if let Some(x) = v.try_downcast_ref::<u32>() {
        Some(Leaf::Number(F::I64, V::I64(*x as i64)))
    } else {
        v.try_downcast_ref::<u64>()
            .map(|x| Leaf::Number(F::I64, V::I64(*x as i64)))
    }
}

/// Build the rows when an [`Inspector<T>`] is added.
fn build_inspector<T: Resource + Reflect>(
    add: On<Add, Inspector<T>>,
    res: Res<T>,
    mut commands: Commands,
) {
    let root = add.entity;
    let mut leaves = Vec::new();
    collect_leaves(res.as_partial_reflect(), "", &mut leaves);

    for (path, leaf) in leaves {
        let row = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    padding: UiRect::vertical(Val::Px(2.0)),
                    ..default()
                },
                ChildOf(root),
            ))
            .id();
        commands.spawn((
            Text::new(path.clone()),
            ThemedText,
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            ChildOf(row),
        ));

        match leaf {
            Leaf::Bool(checked) => {
                let mut widget =
                    commands.spawn_scene(bsn! { @FeathersCheckbox });
                widget.insert((
                    InspectorField::<T>::new(path),
                    // Glass the box (the checkbox's first child).
                    glass::glassify_child(glass::field()),
                    ChildOf(row),
                ));
                if checked {
                    widget.insert(Checked);
                }
            }
            Leaf::Number(format, value) => {
                let widget = commands
                    .spawn_scene(bsn! {
                        @FeathersNumberInput {
                            @number_format: {format}
                        }
                        Node {
                            width: Val::Px(110.0),
                            flex_grow: 0.0,
                        }
                    })
                    .insert((
                        InspectorField::<T>::new(path),
                        // The number input's root is its themed fill
                        // container; glass it directly.
                        glass::field(),
                        ChildOf(row),
                    ))
                    .id();
                // Seed the displayed value.
                commands.trigger(UpdateNumberInput {
                    entity: widget,
                    value,
                });
            }
        }
    }
}

/// Checkbox toggled: write it back and drive the controlled
/// `Checked`.
fn on_change_bool<T: Resource<Mutability = Mutable> + Reflect>(
    change: On<ValueChange<bool>>,
    q_field: Query<&InspectorField<T>>,
    mut res: ResMut<T>,
    mut commands: Commands,
) {
    let Ok(field) = q_field.get(change.source) else {
        return;
    };
    if let Ok(value) =
        res.as_mut().path_mut::<bool>(field.path.as_str())
    {
        *value = change.value;
    }
    if change.value {
        commands.entity(change.source).insert(Checked);
    } else {
        commands.entity(change.source).remove::<Checked>();
    }
}

/// Number input edited: write the value back through the reflect
/// path, casting to the field's concrete numeric type.
fn on_change_number<
    T: Resource<Mutability = Mutable> + Reflect,
    V: NumberValue,
>(
    change: On<ValueChange<V>>,
    q_field: Query<&InspectorField<T>>,
    mut res: ResMut<T>,
) {
    let Ok(field) = q_field.get(change.source) else {
        return;
    };
    let Ok(target) =
        res.as_mut().reflect_path_mut(field.path.as_str())
    else {
        return;
    };
    apply_number(target, change.value.as_f64());
}

/// Numeric event payloads the number input can emit.
trait NumberValue: Copy + Send + Sync + 'static {
    fn as_f64(self) -> f64;
}
macro_rules! impl_number_value {
    ($($ty:ty),*) => {$(
        impl NumberValue for $ty {
            fn as_f64(self) -> f64 {
                self as f64
            }
        }
    )*};
}
impl_number_value!(f32, f64, i32, i64);

fn apply_number(target: &mut dyn PartialReflect, v: f64) {
    if let Some(x) = target.try_downcast_mut::<f32>() {
        *x = v as f32;
    } else if let Some(x) = target.try_downcast_mut::<f64>() {
        *x = v;
    } else if let Some(x) = target.try_downcast_mut::<i32>() {
        *x = v as i32;
    } else if let Some(x) = target.try_downcast_mut::<i64>() {
        *x = v as i64;
    } else if let Some(x) = target.try_downcast_mut::<u32>() {
        *x = v.max(0.0) as u32;
    } else if let Some(x) = target.try_downcast_mut::<u64>() {
        *x = v.max(0.0) as u64;
    }
}

/// Push external resource changes back into the widgets. Focused
/// number inputs ignore [`UpdateNumberInput`], so live edits win.
fn sync_inspector<T: Resource + Reflect>(
    res: Res<T>,
    q_fields: Query<(Entity, &InspectorField<T>, Has<Checked>)>,
    q_formats: Query<&NumberFormat>,
    mut commands: Commands,
) {
    if !res.is_changed() || res.is_added() {
        return;
    }
    for (entity, field, checked) in &q_fields {
        let Ok(target) = res.reflect_path(field.path.as_str()) else {
            continue;
        };
        if let Some(leaf) = as_leaf(target) {
            match leaf {
                Leaf::Bool(value) => {
                    if value != checked {
                        if value {
                            commands.entity(entity).insert(Checked);
                        } else {
                            commands
                                .entity(entity)
                                .remove::<Checked>();
                        }
                    }
                }
                Leaf::Number(_, mut value) => {
                    // Re-express in the widget's own format.
                    if let Ok(format) = q_formats.get(entity) {
                        value = convert(value, *format);
                    }
                    commands
                        .trigger(UpdateNumberInput { entity, value });
                }
            }
        }
    }
}

fn convert(
    value: NumberInputValue,
    format: NumberFormat,
) -> NumberInputValue {
    use NumberInputValue as V;
    let v = match value {
        V::F32(x) => x as f64,
        V::F64(x) => x,
        V::I32(x) => x as f64,
        V::I64(x) => x as f64,
    };
    match format {
        NumberFormat::F32 => V::F32(v as f32),
        NumberFormat::F64 => V::F64(v),
        NumberFormat::I32 => V::I32(v as i32),
        NumberFormat::I64 => V::I64(v as i64),
    }
}
