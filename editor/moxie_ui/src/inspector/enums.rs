//! Picking an enum's variant.
//!
//! Not a registered [`Inspect`](super::Inspect) widget: which variants
//! a type has is something reflection already knows, so this is
//! dispatched on the shape of the value rather than on its type. That
//! is also what lets it serve enums from crates the inspector cannot
//! name.
//!
//! Switching into a variant that carries data means inventing that
//! data: a unit variant needs none, and a variant with fields is
//! constructible when every one of its field types has a registered
//! [`ReflectDefault`]. An enum with any variant that isn't - some
//! field type nothing derives `Default` for - is shown read-only, at
//! whichever variant it is already on, with that variant's fields
//! walked underneath like a struct's.

use bevy::prelude::*;
use bevy::reflect::enums::{
    DynamicEnum, DynamicVariant, VariantInfo,
};
use bevy::reflect::std_traits::ReflectDefault;
use bevy::reflect::structs::DynamicStruct;
use bevy::reflect::tuple::DynamicTuple;
use bevy::reflect::{
    PartialReflect, ReflectRef, TypeInfo, TypeRegistry,
};
use bevy::ui_widgets::Activate;

use bevy_fynix::EntityExt;
use fynix_mock::composer::Composer;
use fynix_mock::ui::ElementHandle;
use fynix_mock::{elem, val};

use super::{Source, when_changed};
use crate::elements::{
    Dropdown, DropdownCursor, DropdownItem, DropdownItemCursor,
    DropdownList, DropdownMenu, Frame, Icon, Label, LabelCursor,
};
use crate::icons;
use crate::motion::MotionExt;
use crate::reactive::{BevyHost, BevyUi};
use crate::theme::EditorTheme;

/// Every variant of `value`'s type, if it is an enum at all.
///
/// Read off the type, not the value, so the choices do not change
/// with whichever variant happens to be active.
pub(super) fn variants(
    value: &dyn PartialReflect,
) -> Option<Vec<String>> {
    if !matches!(value.reflect_ref(), ReflectRef::Enum(_)) {
        return None;
    }

    let TypeInfo::Enum(info) = value.get_represented_type_info()?
    else {
        return None;
    };
    Some(info.variant_names().iter().map(|n| n.to_string()).collect())
}

/// Whether every variant of `value`'s type can be switched into.
pub(super) fn constructible(
    value: &dyn PartialReflect,
    registry: &TypeRegistry,
) -> bool {
    let Some(TypeInfo::Enum(info)) =
        value.get_represented_type_info()
    else {
        return false;
    };

    info.iter().all(|variant| defaultable(variant, registry))
}

/// A unit variant needs nothing; a variant with fields needs every
/// field's type to carry a [`ReflectDefault`].
fn defaultable(
    variant: &VariantInfo,
    registry: &TypeRegistry,
) -> bool {
    match variant {
        VariantInfo::Unit(_) => true,
        VariantInfo::Struct(info) => info
            .iter()
            .all(|field| has_default(field.type_id(), registry)),
        VariantInfo::Tuple(info) => info
            .iter()
            .all(|field| has_default(field.type_id(), registry)),
    }
}

fn has_default(
    type_id: core::any::TypeId,
    registry: &TypeRegistry,
) -> bool {
    registry.get_type_data::<ReflectDefault>(type_id).is_some()
}

/// `name` as a [`DynamicVariant`] of `value`'s type, its fields (if
/// any) filled from their own [`ReflectDefault`]. `None` if the
/// variant isn't constructible - a caller only reaches this from a
/// picker [`constructible`] already gated, so that should not happen.
fn constructed(
    value: &dyn PartialReflect,
    registry: &TypeRegistry,
    name: &str,
) -> Option<DynamicVariant> {
    let TypeInfo::Enum(info) = value.get_represented_type_info()?
    else {
        return None;
    };

    Some(match info.variant(name)? {
        VariantInfo::Unit(_) => DynamicVariant::Unit,
        VariantInfo::Struct(info) => {
            let mut fields = DynamicStruct::default();
            for field in info.iter() {
                let default = registry
                    .get_type_data::<ReflectDefault>(field.type_id())?
                    .default();
                fields.insert_boxed(
                    field.name(),
                    default.into_partial_reflect(),
                );
            }
            DynamicVariant::Struct(fields)
        }
        VariantInfo::Tuple(info) => {
            let mut fields = DynamicTuple::default();
            for field in info.iter() {
                let default = registry
                    .get_type_data::<ReflectDefault>(field.type_id())?
                    .default();
                fields.insert_boxed(default.into_partial_reflect());
            }
            DynamicVariant::Tuple(fields)
        }
    })
}

/// Which variant `source` is on.
fn active(source: &dyn Source, world: &World) -> Option<String> {
    let value = source.get(world)?;
    let ReflectRef::Enum(value) = value.reflect_ref() else {
        return None;
    };
    Some(value.variant_name().to_string())
}

/// The variant, as a dropdown over the rest.
///
/// `pick` is false when some variant of the type isn't
/// [`constructible`], which then only names where it stands. The two
/// look nothing alike, so they share a [`Frame`] and this comes back
/// the same either way.
pub(super) struct VariantPicker<'a> {
    pub source: &'a dyn Source,
    pub variants: Vec<String>,
    pub pick: bool,
}

impl Composer<BevyHost> for VariantPicker<'_> {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        let Self {
            source,
            variants,
            pick,
        } = self;

        let theme = ui.world.resource::<EditorTheme>().clone();
        let current = active(source, ui.world)
            .unwrap_or_else(|| "-".to_string());
        let source = source.boxed();
        // Sized to the longest variant, not the one showing, so
        // picking another does not resize the row.
        let width = Dropdown::width_for(&variants, 12.0);

        ui.elem(elem!(Frame, align = AlignItems::Center))
            .with(move |ui| {
                if !pick {
                    name(ui, &*source, &theme, current);
                    return;
                }

                ui.elem(elem!(DropdownMenu)).with(move |ui| {
                    control(ui, &*source, &theme, current, width);
                    list(ui, &*source, &theme, variants, width);
                });
            })
            .handle()
    }
}

/// Just the active variant, for an enum that cannot be moved.
fn name(
    ui: &mut BevyUi,
    source: &dyn Source,
    theme: &EditorTheme,
    current: String,
) {
    let shown = source.boxed();

    ui.elem(elem!(
        Label,
        text = current,
        wrap = false,
        color = Some(theme.text_primary)
    ))
    .bind(
        |label| label.text(),
        when_changed(source),
        move |world, _| active(&*shown, world).unwrap_or_default(),
    );
}

/// The shut control, showing whichever variant is active.
fn control(
    ui: &mut BevyUi,
    source: &dyn Source,
    theme: &EditorTheme,
    current: String,
    width: Val,
) {
    let shown = source.boxed();

    ui.elem(elem!(
        Dropdown,
        min_width = width,
        max_width = width,
        label = val!(
            Label,
            text = current,
            wrap = false,
            color = Some(theme.text_primary)
        ),
        chevron = val!(
            Icon,
            image = icons::CHEVRON,
            color = theme.text_muted,
            size = px(9),
            rotation = 180.0f32
        )
    ))
    .lit(
        |dropdown| dropdown.fill(),
        theme.hover_overlay,
        theme.hover_overlay,
    )
    .bind(
        |dropdown| dropdown.label().text(),
        when_changed(source),
        move |world, _| active(&*shown, world).unwrap_or_default(),
    );
}

/// One row per variant. The list closes itself once one is picked.
fn list(
    ui: &mut BevyUi,
    source: &dyn Source,
    theme: &EditorTheme,
    variants: Vec<String>,
    width: Val,
) {
    let source = source.boxed();
    let theme = theme.clone();

    ui.elem(elem!(DropdownList, width = width)).with(move |ui| {
        for variant in variants {
            option(ui, &*source, &theme, variant);
        }
    });
}

fn option(
    ui: &mut BevyUi,
    source: &dyn Source,
    theme: &EditorTheme,
    variant: String,
) {
    let chosen = variant.clone();
    let edited = source.boxed();

    ui.elem(elem!(
        DropdownItem,
        label = val!(
            Label,
            text = variant,
            wrap = false,
            color = Some(theme.text_primary)
        )
    ))
    .lit(|item| item.fill(), theme.hover_overlay, theme.hover_overlay)
    .observe(move |_: On<Activate>, mut commands: Commands| {
        let (source, variant) = (edited.boxed(), chosen.clone());

        commands.queue(move |world: &mut World| {
            let Some(value) = source.get(world) else {
                return;
            };
            let dynamic = {
                let registry =
                    world.resource::<AppTypeRegistry>().read();
                constructed(&*value, &registry, &variant)
            };
            if let Some(dynamic) = dynamic {
                source
                    .set(world, &DynamicEnum::new(variant, dynamic));
            }
        });
    });
}
