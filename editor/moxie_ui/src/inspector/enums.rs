//! Picking an enum's variant.
//!
//! Not a registered [`Inspect`](super::Inspect) widget: which variants
//! a type has is something reflection already knows, so this is
//! dispatched on the shape of the value rather than on its type. That
//! is also what lets it serve enums from crates the inspector cannot
//! name.
//!
//! Only unit variants can be picked. Switching into one that carries
//! data would mean inventing that data, so an enum that has any is
//! shown at whichever variant it is already on, with that variant's
//! fields walked underneath like a struct's.

use bevy::prelude::*;
use bevy::reflect::enums::{
    DynamicEnum, DynamicVariant, VariantType,
};
use bevy::reflect::{PartialReflect, ReflectRef, TypeInfo};
use bevy::ui_widgets::Activate;

use bevy_fynix::ElementMutExt;
use fynix_mock::composer::Composer;
use fynix_mock::ui::ElementHandle;
use fynix_mock::{elem, val};

use super::{Source, when_changed};
use crate::elements::{
    Dropdown, DropdownCursor, DropdownItem, DropdownItemCursor,
    DropdownList, DropdownMenu, Frame, Icon, Label, LabelCursor,
};
use crate::icons;
use crate::motion::{HOVER, MotionExt};
use crate::reactive::{BevyHost, BevyUi};
use crate::theme::EditorTheme;

/// What [`Label`] defaults to, which is what the rows are drawn at.
const LABEL_SIZE: f32 = 12.0;

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

/// Whether every variant is a unit one, and so free to pick.
pub(super) fn all_unit(value: &dyn PartialReflect) -> bool {
    let Some(TypeInfo::Enum(info)) =
        value.get_represented_type_info()
    else {
        return false;
    };

    (0..info.variant_len()).all(|index| {
        info.variant_at(index)
            .is_some_and(|v| v.variant_type() == VariantType::Unit)
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
/// `pick` is false for an enum carrying data, which then only names
/// where it stands - moving it would mean inventing that data. The
/// two look nothing alike, so they share a [`Frame`] and this comes
/// back the same either way.
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
        let width = Dropdown::width_for(&variants, LABEL_SIZE);

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
    .lit(|dropdown| dropdown.fill(), HOVER, HOVER)
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
    .lit(|item| item.fill(), HOVER, HOVER)
    .observe(move |_: On<Activate>, mut commands: Commands| {
        let (source, variant) = (edited.boxed(), chosen.clone());

        commands.queue(move |world: &mut World| {
            source.set(
                world,
                &DynamicEnum::new(variant, DynamicVariant::Unit),
            );
        });
    });
}
