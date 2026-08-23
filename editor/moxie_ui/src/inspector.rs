//! Reflection-driven inspector.
//!
//! [`InspectorFields`] walks any reflected value in the world and
//! renders it as a collapsible hierarchy of editable rows. Which
//! widget a leaf gets is a type-registry lookup, not a match on
//! concrete types, so a new editable type is one [`Inspect`] impl
//! away.
//!
//! A widget is handed a [`Source`] rather than a value, and never
//! learns where that value actually lives. [`Field`] (a component of
//! an entity) is the one the walk uses, but anything else the editor
//! keeps can serve the same widgets.

mod enums;
mod field;
mod handle;
mod primitive;
mod text;
mod tree;
mod vector;

use std::any::TypeId;

use bevy::light::CascadeShadowConfig;
use bevy::prelude::*;
use bevy::reflect::{FromType, GetTypeRegistration, PartialReflect};
use fynix_mock::elem;
use moxie_asset::AssetKindAppExt;

use crate::elements::{Frame, Label};
use crate::fold;
use crate::reactive::BevyUi;
pub use field::Field;
pub(crate) use tree::single_value;
pub use tree::{InspectorFields, Section};

/// The widgets and the entity-inspector sections available out of
/// the box.
///
/// Anything else is one [`InspectAppExt::register_inspect`] or
/// [`InspectAppExt::register_inspectable`] away, and needs no change
/// here.
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
            .register_inspect::<Quat>()
            .register_inspect::<String>()
            .register_inspect::<Name>()
            .register_inspect::<Handle<StandardMaterial>>()
            .register_inspect::<Handle<Mesh>>()
            .register_inspectable::<Name>()
            .register_inspectable::<Visibility>()
            .register_inspectable::<Transform>()
            .register_inspectable::<Camera3d>()
            .register_inspectable::<CascadeShadowConfig>()
            .register_inspectable::<DirectionalLight>()
            .register_inspectable::<PointLight>()
            .register_inspectable::<RectLight>()
            .register_inspectable::<SpotLight>()
            .register_inspectable::<Mesh3d>()
            .register_inspectable_as::<MeshMaterial3d<StandardMaterial>>(
                "Standard Material",
            )
            .register_asset_kind::<StandardMaterial>(&["mat"]);
    }
}

/// Registering inspector widgets on the app.
pub trait InspectAppExt {
    /// Makes `T` editable wherever the inspector meets it.
    fn register_inspect<T: Inspect>(&mut self) -> &mut Self;

    /// Makes `T` a section of its own wherever an [`EntityInspector`](
    /// crate::elements::EntityInspector) meets it.
    ///
    /// Opt-in like [`register_inspect`](Self::register_inspect):
    /// `#[reflect(Component)]` lets the inspector reach a value, not
    /// decide it's worth a row. Bevy reflects plenty nobody authors,
    /// like `GlobalTransform`.
    fn register_inspectable<
        T: Component + Reflect + TypePath + GetTypeRegistration,
    >(
        &mut self,
    ) -> &mut Self;

    /// As [`register_inspectable`](Self::register_inspectable), with
    /// `name` heading the section instead of `T`'s own name split
    /// into words.
    fn register_inspectable_as<
        T: Component + Reflect + TypePath + GetTypeRegistration,
    >(
        &mut self,
        name: &'static str,
    ) -> &mut Self;
}

impl InspectAppExt for App {
    fn register_inspect<T: Inspect>(&mut self) -> &mut Self {
        self.register_type::<T>()
            .register_type_data::<T, ReflectInspect>()
    }

    fn register_inspectable<
        T: Component + Reflect + TypePath + GetTypeRegistration,
    >(
        &mut self,
    ) -> &mut Self {
        self.register_type::<T>()
            .register_type_data::<T, ReflectInspectable>()
    }

    fn register_inspectable_as<
        T: Component + Reflect + TypePath + GetTypeRegistration,
    >(
        &mut self,
        name: &'static str,
    ) -> &mut Self {
        self.register_type::<T>();
        let registry =
            self.world().resource::<AppTypeRegistry>().clone();
        let mut registry = registry.write();
        if let Some(registration) =
            registry.get_mut(TypeId::of::<T>())
        {
            registration
                .insert(ReflectInspectable { name: Some(name) });
        }
        self
    }
}

/// Where a widget reads and writes the value it edits.
///
/// Reflected rather than typed, so it can be handed to whichever
/// widget the registry picked for an unknown type.
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

    /// Skips the write if `value` is what the source already holds.
    /// A field commits on blur as well as on edit, which would
    /// otherwise bump the component's tick for nothing.
    fn write<T: PartialReflect>(&self, world: &mut World, value: T) {
        let unchanged = self.get(world).is_some_and(|current| {
            current.reflect_partial_eq(&value).unwrap_or(false)
        });
        if !unchanged {
            self.set(world, &value);
        }
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

/// Fires when `get`'s value differs from the last poll, and on the
/// first poll. For a [`Source::changed`] with no tick to ride:
/// `PartialReflect` has no `PartialEq`, so this compares through
/// [`PartialReflect::reflect_partial_eq`] instead.
pub fn reflect_changed(
    get: impl Fn(&World) -> Option<Box<dyn PartialReflect>>
    + Send
    + Sync
    + 'static,
) -> impl FnMut(&World) -> bool + Send + Sync + 'static {
    let mut seen: Option<Option<Box<dyn PartialReflect>>> = None;
    move |world| {
        let current = get(world);
        let fired = !seen.as_ref().is_some_and(|last| {
            match (last, &current) {
                (Some(last), Some(current)) => last
                    .reflect_partial_eq(&**current)
                    .unwrap_or(false),
                (None, None) => true,
                _ => false,
            }
        });
        seen = Some(current);
        fired
    }
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
        let pick = {
            let registry =
                ui.world.resource::<AppTypeRegistry>().read();
            enums::constructible(&*value, &registry)
        };
        ui.compose(enums::VariantPicker {
            source,
            variants,
            pick,
        });
    }
}

/// One field's row: a label column, then whatever `value` builds
/// beside it. The split is proportional (40/60), not a fixed pixel
/// width, so it scales with however wide the panel is docked - the
/// same convention Unity, Godot, and Unreal's own inspectors use.
///
/// `depth` is how many [`Foldable`](crate::fold::Foldable) bodies
/// this row sits under. Each one narrows the row by its own indent,
/// which would otherwise pull the 40% mark inward with it; the label
/// sheds that same width back so `value` starts at the same place
/// no matter how deep its row is nested.
pub(crate) fn field_row(
    ui: &mut BevyUi,
    label: String,
    color: Color,
    bold: bool,
    depth: u32,
    value: impl FnOnce(&mut BevyUi),
) {
    const VALUE_SHARE: f32 = 0.6;
    const LABEL_SIZE: f32 = 12.0;
    const EDGE_PADDING: f32 = 8.0;
    let indent = ui.theme.fold_indent + fold::RAIL_WIDTH;
    let shed = VALUE_SHARE * depth as f32 * indent;

    ui.elem(elem!(
        Frame,
        width = percent(100),
        direction = FlexDirection::Row,
        align = AlignItems::Center,
        column_gap = px(8),
        padding = UiRect::vertical(px(3))
    ))
    .with(move |ui| {
        ui.elem(elem!(
            Frame,
            width = percent(40),
            margin = UiRect::right(px(-shed)),
            overflow = Overflow::clip_x(),
            padding = UiRect::right(px(EDGE_PADDING))
        ))
        .with(move |ui| {
            ui.elem(elem!(
                Label,
                text = label,
                size = LABEL_SIZE,
                color = Some(color),
                bold = bold,
                wrap = false
            ));
        });
        ui.elem(elem!(Frame, flex_grow = 1.0f32)).with(value);
    });
}

/// Builds the editing widget for one reflected value.
///
/// The value itself is not passed in. A widget is built once, then
/// binds to its source and re-reads whenever that fires, so a
/// focused input survives an edit.
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

/// Marks a component as one an [`EntityInspector`](
/// crate::elements::EntityInspector) shows. See
/// [`InspectAppExt::register_inspectable`].
#[derive(Clone)]
pub struct ReflectInspectable {
    /// What the section is headed with instead of the type's own
    /// name, split into words. See
    /// [`InspectAppExt::register_inspectable_as`].
    pub name: Option<&'static str>,
}

impl<T: Component + Reflect> FromType<T> for ReflectInspectable {
    fn from_type() -> Self {
        Self { name: None }
    }
}
