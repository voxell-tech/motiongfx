//! "+" button popup: lists registered windows; clicking one adds it
//! as a tab to that area's leaf.
//!
//! State-driven: the click observer only writes
//! [`AddWindowPopupState`]; a watcher renders whatever that says.
//! The state lives on the overlay node itself (there's exactly one),
//! rather than a global `Resource`; see [`crate::reactive::component_changed`].

use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy::ui_widgets::Activate;

use super::area::DockTabAddButton;
use super::reconcile::NodeBinding;
use super::registry::WindowRegistry;
use super::tree::DockTree;
use crate::layout::logical_rect;
use bevy_fynix::WorldEntityMut;
use fynix::{elem, val};

use crate::elements::{Frame, GhostButton, Icon, Label, Overlay};
use crate::icons;
use crate::reactive::{BevyUi, component_changed};

const POPUP_WIDTH: f32 = 150.0;

/// The open popup, if any: which "+" button owns it and where it sits.
/// One overlay node ever carries this, so a plain `Query::single_mut`
/// finds it from anywhere.
#[derive(Component, Default, PartialEq, Clone)]
pub struct AddWindowPopupState {
    open: Option<OpenPopup>,
}

#[derive(PartialEq, Clone)]
struct OpenPopup {
    owner: Entity,
    area: Entity,
    left: f32,
    top: f32,
}

/// The popup, as kernel nodes. Rebuilds when the state changes, which
/// covers opening, closing, and moving between buttons.
pub(super) fn add_window_popup(ui: &mut BevyUi) {
    // A full-window overlay, because the popup positions itself in
    // window coordinates and an absolute child positions against its
    // parent. It is always there, so it catches nothing.
    ui.elem(elem!(Overlay))
        .insert(AddWindowPopupState::default())
        .watch(
            component_changed::<AddWindowPopupState>(),
            build_popup,
        );
}

/// Open the popup under the clicked "+" button; clicking the same
/// button again closes it.
pub(super) fn on_add_click(
    mut click: On<Pointer<Click>>,
    q_buttons: Query<(
        &DockTabAddButton,
        &ComputedNode,
        &UiGlobalTransform,
    )>,
    mut q_state: Query<&mut AddWindowPopupState>,
) {
    click.propagate(false);
    let owner = click.entity;
    let Ok((button, computed, transform)) = q_buttons.get(owner)
    else {
        return;
    };
    let Ok(mut state) = q_state.single_mut() else {
        return;
    };

    if state.open.as_ref().is_some_and(|open| open.owner == owner) {
        state.open = None;
        return;
    }

    // Right-aligned to the button, just below it.
    let rect = logical_rect(computed, transform);
    state.open = Some(OpenPopup {
        owner,
        area: button.area_entity,
        left: rect.max.x - POPUP_WIDTH,
        top: rect.max.y + 4.0,
    });
}

/// What a row adds when it is picked, so the handler is a system
/// rather than a closure holding the two.
#[derive(Component)]
struct AddsWindow {
    area: Entity,
    window_id: String,
}

/// Add the row's window to its area, and close the popup.
fn on_pick(
    pick: On<Activate>,
    rows: Query<&AddsWindow>,
    areas: Query<&NodeBinding>,
    mut tree: ResMut<DockTree>,
    mut popup: Query<&mut AddWindowPopupState>,
) {
    let Ok(row) = rows.get(pick.entity) else {
        return;
    };

    if let Ok(binding) = areas.get(row.area) {
        tree.add_tab(binding.0, row.window_id.clone());
    }
    if let Ok(mut state) = popup.single_mut() {
        state.open = None;
    }
}

/// Close on any click that isn't on the popup itself.
fn close_popup(
    mut click: On<Pointer<Click>>,
    mut q_state: Query<&mut AddWindowPopupState>,
) {
    click.propagate(false);
    if let Ok(mut state) = q_state.single_mut() {
        state.open = None;
    }
}

fn build_popup(ui: &mut BevyUi) {
    let popup_root = ui.parent();
    let Some(open) = ui
        .world
        .get::<AddWindowPopupState>(popup_root)
        .and_then(|state| state.open.clone())
    else {
        return;
    };

    // Catches the click outside, but lets hover and clicks through to
    // the UI beneath rather than freezing it.
    ui.elem(elem!(Overlay, catches = true, z = 180))
        .observe(close_popup);

    let (left, top, area) = (open.left, open.top, open.area);

    ui.elem(elem!(
        Frame,
        position = PositionType::Absolute,
        inset = UiRect::new(px(left), auto(), px(top), auto(),),
        width = px(POPUP_WIDTH),
        direction = FlexDirection::Column,
        row_gap = px(2),
        padding = UiRect::all(px(4)),
        radius = px(6),
        background = Color::srgba(0.11, 0.10, 0.11, 0.98),
        z = 181
    ))
    .with(move |ui| build_rows(ui, area));
}

/// Windows are single-instance, so only closed ones are listed.
fn build_rows(ui: &mut BevyUi, area: Entity) {
    let text_color = ui.theme.text_primary;
    let tree = ui.world.resource::<DockTree>();
    let closed = ui
        .world
        .resource::<WindowRegistry>()
        .iter()
        .filter(|d| tree.find_leaf_with_window(&d.id).is_none())
        .map(|d| (d.id.clone(), d.name.clone(), d.icon.clone()))
        .collect::<Vec<_>>();

    // Every window is already open, so the popup would be a blank
    // box: say why it is empty rather than showing nothing.
    if closed.is_empty() {
        let muted = ui.theme.text_muted;

        ui.elem(elem!(
            Label,
            text = "Nothing left to add",
            color = muted
        ));
        return;
    }

    for (window_id, name, icon) in closed {
        let (image, icon_color) = match icon {
            Some(icon) => (icon, text_color),
            None => (
                icons::PLACEHOLDER.to_string(),
                text_color.with_alpha(0.0),
            ),
        };

        ui.elem(elem!(
            !GhostButton,
            width = percent(100),
            height = auto(),
            justify = JustifyContent::FlexStart,
            icon = val!(
                Icon,
                image = image,
                color = icon_color,
                size = px(12)
            ),
            label = val!(Label, text = name, color = text_color)
        ))
        .insert(AddsWindow { area, window_id })
        .observe(on_pick);
    }
}
