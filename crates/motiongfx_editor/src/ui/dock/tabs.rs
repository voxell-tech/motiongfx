//! Tab bar widget: spawns a `DockTabBar` (row of `DockTab`s + an "add
//! tab" button) for a leaf, and keeps click handling for
//! activating/closing tabs.

use bevy::feathers::constants::icons;
use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui::widget::ImageNode;
use bevy::window::SystemCursorIcon;

use super::area::{DockTab, DockTabBar, DockTabCloseButton};
use super::reconcile::LeafBinding;
use super::tree::{DockTree, TabId};
use super::{
    PANEL_BORDER, PANEL_HEADER_BG, TAB_ACTIVE_BG, TAB_ACTIVE_BORDER, TAB_HEIGHT,
    TAB_INACTIVE_TEXT, TEXT_MAIN,
};

#[derive(Component)]
pub struct DockTabAddButton {
    pub area_entity: Entity,
}

#[derive(Component)]
pub struct DockTabRow;

pub struct DockTabPlugin;

impl Plugin for DockTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_dock_tab_clicks,
                handle_close_clicks,
                show_close_on_hover,
            ),
        );
    }
}

pub fn spawn_tab_bar_world(
    world: &mut World,
    area_entity: Entity,
    tabs: &[(TabId, String, String)],
    active: Option<TabId>,
) {
    let tab_bar = world
        .spawn((
            DockTabBar,
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                height: Val::Px(TAB_HEIGHT),
                padding: UiRect::new(Val::Px(8.0), Val::Px(8.0), Val::Px(1.0), Val::ZERO),
                flex_shrink: 0.0,
                border: UiRect {
                    left: Val::Px(1.0),
                    right: Val::Px(1.0),
                    top: Val::Px(1.0),
                    bottom: Val::ZERO,
                },
                border_radius: BorderRadius::top(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(PANEL_HEADER_BG),
            BorderColor::all(PANEL_BORDER),
            ChildOf(area_entity),
        ))
        .id();

    let tab_row = world
        .spawn((
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
            ChildOf(tab_bar),
        ))
        .id();

    for (tab_id, window_id, label) in tabs {
        let is_active = Some(*tab_id) == active;
        spawn_tab(world, tab_row, *tab_id, window_id, label, is_active);
    }

    world.spawn((
        DockTabAddButton { area_entity },
        Interaction::default(),
        EntityCursor::System(SystemCursorIcon::Pointer),
        Node {
            width: Val::Px(18.0),
            height: Val::Px(18.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_shrink: 0.0,
            ..default()
        },
        ChildOf(tab_bar),
        children![(
            Text::new("+"),
            TextFont {
                font_size: FontSize::Px(11.0),
                ..default()
            },
            TextColor(TAB_INACTIVE_TEXT),
        )],
    ));
}

fn spawn_tab(
    world: &mut World,
    tab_row: Entity,
    tab_id: TabId,
    window_id: &str,
    label: &str,
    is_active: bool,
) {
    let tab_bg = if is_active { TAB_ACTIVE_BG } else { Color::NONE };
    let border_top = if is_active { Val::Px(2.0) } else { Val::ZERO };
    let border_color = if is_active { TAB_ACTIVE_BORDER } else { Color::NONE };
    let text_color = if is_active { TEXT_MAIN } else { TAB_INACTIVE_TEXT };

    let tab_entity = world
        .spawn((
            DockTab {
                window_id: window_id.to_string(),
                tab_id,
            },
            Interaction::default(),
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(4.0),
                padding: UiRect::horizontal(Val::Px(5.0)),
                height: Val::Percent(100.0),
                flex_shrink: 0.0,
                border: UiRect {
                    top: border_top,
                    ..default()
                },
                // Slightly rounded, matching the timeline's clip boxes.
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(tab_bg),
            BorderColor::all(border_color),
            // Tabs are draggable: signal it on hover.
            EntityCursor::System(SystemCursorIcon::Grab),
            ChildOf(tab_row),
        ))
        .id();

    world.spawn((
        Text::new(label.to_string()),
        TextLayout::linebreak(LineBreak::NoWrap),
        TextFont {
            font_size: FontSize::Px(12.0),
            weight: FontWeight::BOLD,
            ..default()
        },
        TextColor(text_color),
        ChildOf(tab_entity),
    ));

    // Close-button slot always reserves its 14x14 layout space so the
    // tab doesn't reflow on hover. The icon inside is alpha-toggled by
    // `show_close_on_hover`.
    let close_icon: Handle<Image> = world.resource::<AssetServer>().load(icons::X);
    world.spawn((
        DockTabCloseButton {
            window_id: window_id.to_string(),
            tab_id,
        },
        Interaction::default(),
        EntityCursor::System(SystemCursorIcon::Pointer),
        Node {
            width: Val::Px(14.0),
            height: Val::Px(14.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(Val::Px(2.0)),
            ..default()
        },
        ChildOf(tab_entity),
        children![(
            DockTabCloseIcon,
            ImageNode {
                image: close_icon,
                color: TAB_INACTIVE_TEXT.with_alpha(0.0),
                ..default()
            },
            Node {
                width: Val::Px(10.0),
                height: Val::Px(10.0),
                ..default()
            },
        )],
    ));
}

/// Marker on the inner close icon of a dock tab close button so the
/// hover system can fade it in / out without reflowing the tab.
#[derive(Component)]
pub struct DockTabCloseIcon;

fn handle_dock_tab_clicks(
    tab_query: Query<(&DockTab, &Interaction, &ChildOf), Changed<Interaction>>,
    parent_query: Query<&ChildOf>,
    bindings: Query<&LeafBinding>,
    mut tree: ResMut<DockTree>,
) {
    for (tab, interaction, tab_child_of) in tab_query.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Walk: tab -> tab_row -> tab_bar -> area
        let tab_row = tab_child_of.parent();
        let Ok(row_parent) = parent_query.get(tab_row) else {
            continue;
        };
        let tab_bar = row_parent.parent();
        let Ok(bar_parent) = parent_query.get(tab_bar) else {
            continue;
        };
        let area_entity = bar_parent.parent();

        let Ok(binding) = bindings.get(area_entity) else {
            continue;
        };

        tree.set_active(binding.0, tab.tab_id);
    }
}

/// Close a tab: remove it from the tree; `simplify` collapses the
/// leaf if it goes empty, and the reconciler tears the UI down.
fn handle_close_clicks(
    q_close: Query<
        (&DockTabCloseButton, &Interaction),
        Changed<Interaction>,
    >,
    mut tree: ResMut<DockTree>,
) {
    for (button, interaction) in &q_close {
        if *interaction == Interaction::Pressed {
            // Keep at least one tab alive across the whole layout.
            if tree.tabs().count() <= 1 {
                continue;
            }
            tree.remove_tab(button.tab_id);
        }
    }
}

fn show_close_on_hover(
    tabs: Query<(Entity, &Interaction, &Children), (Changed<Interaction>, With<DockTab>)>,
    drag_state: Option<Res<super::drag::DockDragState>>,
    close_buttons: Query<&Children, With<DockTabCloseButton>>,
    mut icon_colors: Query<&mut ImageNode, With<DockTabCloseIcon>>,
) {
    let hide =
        drag_state.is_none_or(|s| matches!(*s, super::drag::DockDragState::Dragging { .. }));

    for (_tab_entity, interaction, children) in tabs.iter() {
        let show =
            (*interaction == Interaction::Hovered || *interaction == Interaction::Pressed) && !hide;
        let alpha = if show { 1.0 } else { 0.0 };
        for child in children.iter() {
            let Ok(close_children) = close_buttons.get(child) else {
                continue;
            };
            for grandchild in close_children.iter() {
                if let Ok(mut image) = icon_colors.get_mut(grandchild) {
                    image.color = image.color.with_alpha(alpha);
                }
            }
        }
    }
}
