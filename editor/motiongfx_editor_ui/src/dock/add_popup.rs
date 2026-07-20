//! "+" button popup: lists registered windows; clicking one adds it
//! as a tab to that area's leaf.
//!
//! State-driven rather than spawned on click. The click observer only
//! writes [`AddWindowPopupState`]; a watcher renders whatever that
//! says. That is the direction the kernel wants: input sends state to
//! the world, the UI derives itself from state.

use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use super::area::DockTabAddButton;
use super::drag::logical_rect;
use super::reconcile::NodeBinding;
use super::registry::WindowRegistry;
use super::tree::DockTree;
use crate::glass::{Glass, glass_button};
use crate::reactive::{BevyUi, value_changed, widget};
use crate::theme::EditorTheme;

const POPUP_WIDTH: f32 = 150.0;

/// The open popup, if any: which "+" button owns it and where it sits.
#[derive(Resource, Default, PartialEq, Clone)]
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

/// Full-screen click-catcher behind the popup.
#[derive(Component, Default, Clone)]
pub struct AddWindowPopupBackdrop;

/// The popup, as kernel nodes. Rebuilds when the state changes, which
/// covers opening, closing, and moving between buttons.
pub(super) fn add_window_popup(ui: &mut BevyUi) {
    ui.watch(
        value_changed(|world: &World, _| {
            world.resource::<AddWindowPopupState>().clone()
        }),
        build_popup,
    )
    // A full-window overlay, because the popup positions itself in
    // window coordinates and an absolute child positions against its
    // parent. `IGNORE` so it doesn't swallow every click meant for
    // the dock underneath.
    .widget(widget(bsn! {
        Pickable::IGNORE
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
        }
    }));
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
    mut state: ResMut<AddWindowPopupState>,
) {
    click.propagate(false);
    let owner = click.entity;
    let Ok((button, computed, transform)) = q_buttons.get(owner)
    else {
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

/// Close on any click that isn't on the popup itself.
fn close_popup(
    mut click: On<Pointer<Click>>,
    mut state: ResMut<AddWindowPopupState>,
) {
    click.propagate(false);
    state.open = None;
}

fn build_popup(ui: &mut BevyUi) {
    let Some(open) =
        ui.world().resource::<AddWindowPopupState>().open.clone()
    else {
        return;
    };

    // Catches the outside-click, but lets hover/clicks through to the
    // UI beneath rather than freezing it.
    ui.node(widget(bsn! {
        AddWindowPopupBackdrop
        on(close_popup)
        Pickable {
            should_block_lower: false,
            is_hoverable: true,
        }
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
        }
        GlobalZIndex(180)
    }));

    let (left, top, area) = (open.left, open.top, open.area);
    ui.node(widget(bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px({left}),
            top: Val::Px({top}),
            width: Val::Px(POPUP_WIDTH),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(4.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
        }
        template_value(Glass::Popup)
        GlobalZIndex(181)
    }))
    .with(move |ui| build_rows(ui, area));
}

/// Windows are single-instance, so only closed ones are listed.
fn build_rows(ui: &mut BevyUi, area: Entity) {
    let text_color =
        ui.world().resource::<EditorTheme>().text_primary;
    let tree = ui.world().resource::<DockTree>();
    let closed = ui
        .world()
        .resource::<WindowRegistry>()
        .iter()
        .filter(|d| tree.find_leaf_with_window(&d.id).is_none())
        .map(|d| (d.id.clone(), d.name.clone()))
        .collect::<Vec<_>>();

    for (window_id, name) in closed {
        // The click handler captures the window id + target area
        // directly instead of going through a component (which would
        // need `Entity`'s absent `Default` for the template system).
        ui.node(widget(bsn! {
            glass_button()
            on(move |mut click: On<Pointer<Click>>,
                     q_bindings: Query<&NodeBinding>,
                     mut tree: ResMut<DockTree>,
                     mut state: ResMut<AddWindowPopupState>| {
                click.propagate(false);
                if let Ok(binding) = q_bindings.get(area) {
                    tree.add_tab(binding.0, window_id.clone());
                }
                state.open = None;
            })
            Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
            }
        }))
        .with(move |ui| {
            ui.node(widget(bsn! {
                Text({name})
                TextFont { font_size: FontSize::Px(12.0) }
                TextColor({text_color})
            }));
        });
    }
}
