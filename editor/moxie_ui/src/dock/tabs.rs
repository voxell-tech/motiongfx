//! Tab bar widget: a `DockTabBar` (row of `DockTab`s + an "add tab"
//! button) for a leaf, built as kernel nodes.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::widget::ImageNode;

use super::TAB_HEIGHT;
use super::area::{
    DockTab, DockTabAddButton, DockTabBar, DockTabCloseButton,
};
use super::tree::{DockNode, DockTree, NodeId, TabId};
use crate::glass::Glass;
use crate::reactive::{BevyUi, BevyUiExt, resource_changed};
use crate::theme::EditorTheme;

/// Hover feedback for a tab: the resting pill swap, and the close
/// icon fading in.
///
/// Driven by [`Hovered`] rather than `Pointer<Over>`/`Out`: it already
/// accounts for descendants, so crossing from the tab's label onto its
/// close button doesn't read as leaving the tab. It's immutable, so
/// every replacement fires `Insert`.
pub(super) fn on_tab_hover(
    insert: On<Insert, Hovered>,
    q_tabs: Query<(&Hovered, &Glass, &Children), With<DockTab>>,
    drag_state: Res<super::drag::DockDragState>,
    close_buttons: Query<&Children, With<DockTabCloseButton>>,
    mut icons: Query<&mut ImageNode, With<DockTabCloseIcon>>,
    mut commands: Commands,
) {
    let tab = insert.entity;
    let Ok((hovered, glass, children)) = q_tabs.get(tab) else {
        return;
    };
    let hovered = hovered.get();

    // Swap the resting pill for the faint hover one. Re-inserting
    // [`Glass`] triggers the material swap; active tabs keep
    // [`Glass::TabActive`].
    let next = match (hovered, glass) {
        (false, Glass::TabHover) => Some(Glass::TabIdle),
        (true, Glass::TabIdle) => Some(Glass::TabHover),
        _ => None,
    };
    if let Some(next) = next {
        commands.entity(tab).insert(next);
    }

    // The close icon is alpha-toggled rather than shown/hidden so the
    // tab never reflows. Stays hidden mid-drag.
    let dragging = matches!(
        *drag_state,
        super::drag::DockDragState::Dragging { .. }
    );
    let alpha = if hovered && !dragging { 1.0 } else { 0.0 };
    for child in children.iter() {
        let Ok(close_children) = close_buttons.get(child) else {
            continue;
        };
        for grandchild in close_children.iter() {
            if let Ok(mut image) = icons.get_mut(grandchild) {
                image.color = image.color.with_alpha(alpha);
            }
        }
    }
}

#[derive(Component)]
pub struct DockTabRow;

/// Build a leaf's tab bar as kernel nodes.
///
/// `area` is passed in rather than found by walking up: the caller
/// just spawned it, so it has the handle. Each kernel node *is* the
/// widget, so the hierarchy stays `tab -> row -> bar -> area`, which
/// drag hit-testing walks.
pub(super) fn build_tab_bar(
    leaf: NodeId,
    area: Entity,
    tabs: Vec<(TabId, String, String)>,
    ui: &mut BevyUi,
) {
    ui.node(|world, node| {
        world.entity_mut(node).insert((
            DockTabBar,
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                height: Val::Px(TAB_HEIGHT),
                // No left padding: first tab sits flush to the edge.
                padding: UiRect::new(
                    Val::ZERO,
                    Val::Px(8.0),
                    Val::Px(1.0),
                    Val::ZERO,
                ),
                flex_shrink: 0.0,
                ..default()
            },
            Glass::Bar,
        ));
    })
    .with(move |ui| {
        ui.node(|world, node| {
            world.entity_mut(node).insert((
                DockTabRow,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(2.0),
                    height: Val::Percent(100.0),
                    overflow: Overflow::scroll_x(),
                    flex_shrink: 1.0,
                    min_width: Val::Px(0.0),
                    ..default()
                },
                ScrollPosition::default(),
            ));
        })
        .with(move |ui| {
            for (tab_id, window_id, label) in tabs {
                build_tab(leaf, area, tab_id, window_id, label, ui);
            }
        });

        let muted = ui.world().resource::<EditorTheme>().text_muted;
        ui.bsn(bsn! {
            @DockTabAddButton {
                @area: {area},
                @icon_color: {muted},
            }
        });
    });
}

/// One tab. Active styling is a binding: switching tabs must not
/// rebuild, or the drag in progress and the bar's scroll offset die
/// with it.
fn build_tab(
    leaf: NodeId,
    area: Entity,
    tab_id: TabId,
    window_id: String,
    label: String,
    ui: &mut BevyUi,
) {
    let is_active = active_of(ui.world(), leaf) == Some(tab_id);
    let (text_color, close_color) = {
        let theme = ui.world().resource::<EditorTheme>();
        let text = if is_active {
            theme.text_primary
        } else {
            theme.text_muted
        };
        (text, theme.text_muted.with_alpha(0.0))
    };

    ui.bsn(bsn! {
        @DockTab {
            @window_id: {window_id},
            @tab_id: {tab_id},
            @label: {label},
            @area: {area},
            @is_active: {is_active},
            @text_color: {text_color},
            @close_color: {close_color},
        }
    })
    .bind_raw(
        resource_changed::<DockTree>(),
        move |world, node| {
            let is_active = active_of(world, leaf) == Some(tab_id);
            let color = {
                let theme = world.resource::<EditorTheme>();
                if is_active {
                    theme.text_primary
                } else {
                    theme.text_muted
                }
            };
            // Re-inserting the preset swaps the tab's material.
            world.entity_mut(node).insert(Glass::tab(is_active));
            let children = world
                .get::<Children>(node)
                .map(|children| children.to_vec())
                .unwrap_or_default();
            for child in children {
                if let Some(mut text) =
                    world.get_mut::<TextColor>(child)
                {
                    text.0 = color;
                }
            }
        },
    );
}

fn active_of(world: &World, leaf: NodeId) -> Option<TabId> {
    match world.resource::<DockTree>().get(leaf) {
        Some(DockNode::Leaf(leaf)) => leaf.active,
        _ => None,
    }
}

/// The shared tab-tile layout (pill body: label + close slot). Used
/// by real tabs and the drag ghost so they're pixel-identical.
pub(super) fn tab_tile_node() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        column_gap: Val::Px(4.0),
        padding: UiRect::horizontal(Val::Px(8.0)),
        height: Val::Percent(100.0),
        flex_shrink: 0.0,
        ..default()
    }
}

/// A drag-ghost copy of a tab tile: the same body + label, plus an
/// inert close-slot spacer so its width matches a real tab. `wrapper`
/// supplies the position + height (see [`super::drag`]).
///
/// Spawned imperatively: the ghost is drag state, not part of the
/// kernel's tree.
pub(super) fn spawn_ghost_tab(
    world: &mut World,
    wrapper: Entity,
    label: &str,
) {
    let color = world.resource::<EditorTheme>().text_primary;
    let tile = world
        .spawn((tab_tile_node(), Glass::tab(true), ChildOf(wrapper)))
        .id();
    world.spawn((
        Text::new(label.to_string()),
        TextLayout::linebreak(LineBreak::NoWrap),
        TextFont {
            font_size: FontSize::Px(12.0),
            weight: FontWeight::BOLD,
            ..default()
        },
        TextColor(color),
        ChildOf(tile),
    ));
    // Matches the 14px close slot a real tab reserves.
    world.spawn((
        Node {
            width: Val::Px(14.0),
            height: Val::Px(14.0),
            ..default()
        },
        ChildOf(tile),
    ));
}

/// Marker on the inner close icon of a dock tab close button so the
/// hover observer can fade it in / out without reflowing the tab.
#[derive(Component, Default, Clone)]
pub struct DockTabCloseIcon;
