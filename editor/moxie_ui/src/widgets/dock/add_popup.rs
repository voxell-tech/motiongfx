//! "+" button popup: lists registered windows; clicking one adds it
//! as a tab to that area's leaf.
//!
//! State-driven: the click observer only writes
//! [`AddWindowPopupState`]; a watcher renders whatever that says.
//! The state lives on the overlay node itself (there's exactly one),
//! rather than a global `Resource` — see [`crate::reactive::component_changed`].

use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use super::area::DockTabAddButton;
use super::drag::logical_rect;
use super::reconcile::NodeBinding;
use super::registry::WindowRegistry;
use super::tree::DockTree;
use crate::elements::Frame;
use crate::glass::{Glass, glass_button};
use crate::icons;
use crate::reactive::{BevyUi, BevyUiExt, component_changed};
use crate::theme::EditorTheme;

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

/// Full-screen click-catcher behind the popup.
#[derive(Component, Default, Clone)]
pub struct AddWindowPopupBackdrop;

/// The popup, as kernel nodes. Rebuilds when the state changes, which
/// covers opening, closing, and moving between buttons.
pub(super) fn add_window_popup(ui: &mut BevyUi) {
    // A full-window overlay, because the popup positions itself in
    // window coordinates and an absolute child positions against its
    // parent. `IGNORE` so it doesn't swallow every click meant for
    // the dock underneath.
    ui.bsn(bsn! {
        Pickable::IGNORE
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
        }
        AddWindowPopupState
    })
    .watch(component_changed::<AddWindowPopupState>(), build_popup);
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
        .world()
        .get::<AddWindowPopupState>(popup_root)
        .and_then(|state| state.open.clone())
    else {
        return;
    };

    // Catches the outside-click, but lets hover/clicks through to the
    // UI beneath rather than freezing it.
    ui.bsn(bsn! {
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
    });

    let (left, top, area) = (open.left, open.top, open.area);
    ui.bsn(bsn! {
        @Frame {
            @width: {Val::Px(POPUP_WIDTH)},
            @direction: {FlexDirection::Column},
            @padding: {UiRect::all(Val::Px(4.0))},
            @radius: {Val::Px(6.0)},
            @glass: {Some(Glass::Popup)},
        }
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px({left}),
            top: Val::Px({top}),
        }
        GlobalZIndex(181)
    })
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
        .map(|d| (d.id.clone(), d.name.clone(), d.icon.clone()))
        .collect::<Vec<_>>();

    for (window_id, name, icon) in closed {
        // The click handler captures the window id + target area
        // directly instead of going through a component (which would
        // need `Entity`'s absent `Default` for the template system).
        ui.bsn(bsn! {
            glass_button()
            on(move |mut click: On<Pointer<Click>>,
                     q_bindings: Query<&NodeBinding>,
                     mut tree: ResMut<DockTree>,
                     mut q_state: Query<&mut AddWindowPopupState>| {
                click.propagate(false);
                if let Ok(binding) = q_bindings.get(area) {
                    tree.add_tab(binding.0, window_id.clone());
                }
                if let Ok(mut state) = q_state.single_mut() {
                    state.open = None;
                }
            })
            @Frame {
                @width: {Val::Percent(100.0)},
                @justify: {JustifyContent::FlexStart},
                @align: {AlignItems::Center},
                @padding: {UiRect::axes(Val::Px(8.0), Val::Px(4.0))},
                @radius: {Val::Px(4.0)},
            }
        })
        .with(move |ui| {
            let (icon_src, icon_color) = match &icon {
                Some(icon) => (icon.clone(), text_color),
                None => (
                    icons::PLACEHOLDER.to_string(),
                    text_color.with_alpha(0.0),
                ),
            };
            ui.bsn(bsn! {
                ImageNode {
                    image: {icon_src},
                    color: {icon_color},
                }
                Node {
                    width: Val::Px(12.0),
                    height: Val::Px(12.0),
                    margin: UiRect::right(Val::Px(6.0)),
                }
            });
            ui.bsn(bsn! {
                Text({name})
                TextFont { font_size: FontSize::Px(12.0) }
                TextColor({text_color})
            });
        });
    }
}
