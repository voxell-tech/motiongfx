//! Tab bar widget: a row of `DockTab`s + an "add tab" button for a
//! leaf, built as kernel nodes.

use bevy::feathers::constants::icons as feathers_icons;
use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy_fynix::EntityExt;
use fynix_mock::{elem, val};
use fynix_mock::WorldNodeRef;

use super::area::DockTabAddButton;
use super::tree::{DockNode, DockTree, NodeId, TabId};
use crate::elements::{
    ButtonElemCursor, GhostButton, Icon, IconCursor, Label,
    LabelCursor, Tab, TabBar, TabCursor, TabRow, TintButton,
};
use crate::icons;
use crate::motion::MotionExt;
use crate::reactive::{BevyUi, resource_changed};

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
    tabs: Vec<(TabId, String, String, Option<String>)>,
    ui: &mut BevyUi,
) {
    ui.elem(elem!(TabBar)).with(move |ui| {
        ui.elem(elem!(TabRow)).with(move |ui| {
            for (tab_id, window_id, label, icon) in tabs {
                build_tab(
                    leaf, area, tab_id, window_id, label, icon, ui,
                );
            }
        });

        build_add_button(area, ui);
    });
}

/// The "+" at the end of the bar, which opens the window list.
fn build_add_button(area: Entity, ui: &mut BevyUi) {
    let muted = ui.theme.text_muted;

    ui.elem(elem!(
        !TintButton::default(),
        icon = val!(Icon, image = icons::PLUS, color = muted)
    ))
    .insert(DockTabAddButton { area_entity: area })
    .observe(super::add_popup::on_add_click);
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
    icon: Option<String>,
    ui: &mut BevyUi,
) {
    let is_active = active_of(ui.world, leaf) == Some(tab_id);
    let theme = ui.theme;
    let primary = theme.text_primary;
    let muted = theme.text_muted;
    let lit = text_color(ui.world, leaf, tab_id, primary, muted);
    let close_color = muted;
    let close_hover = theme.critical;

    let mut tab = ui.elem(elem!(
        Tab,
        window_id = window_id,
        tab = tab_id,
        active = is_active,
        icon = icon.map(|image| val!(
            Icon,
            image = image,
            color = lit,
            size = px(12)
        )),
        label = val!(
            Label,
            text = label,
            color = Some(lit),
            bold = true,
            wrap = false
        ),
        close = val!(
            !GhostButton,
            width = px(14),
            height = px(14),
            padding = UiRect::ZERO,
            radius = px(2),
            icon = val!(
                Icon,
                image = feathers_icons::X,
                color = close_color,
                size = px(10)
            )
        )
    ));
    tab
        // Which tab is active follows the tree, and must not rebuild the
        // tab: a drag in progress would go with it.
        .bind(
            |tab| tab.active(),
            resource_changed::<DockTree>(),
            move |WorldNodeRef { world, .. }| active_of(world, leaf) == Some(tab_id),
        )
        // What the tab holds is lit by the same signal, and separately:
        // the fill is the tab's own field, these are its children's.
        .bind(
            |tab| tab.label().color(),
            resource_changed::<DockTree>(),
            move |WorldNodeRef { world, .. }| {
                text_color(world, leaf, tab_id, primary, muted)
            },
        )
        .bind(
            |tab| tab.icon().color(),
            resource_changed::<DockTree>(),
            move |WorldNodeRef { world, .. }| {
                text_color(world, leaf, tab_id, primary, muted)
            },
        )
        .observe(
            move |mut click: On<Pointer<Click>>,
                  bindings: Query<&super::reconcile::LeafBinding>,
                  mut tree: ResMut<DockTree>| {
                click.propagate(false);

                if let Ok(binding) = bindings.get(area) {
                    tree.set_active(binding.0, tab_id);
                }
            },
        );

    // On the close button itself: a `Button` takes the click for
    // itself, so nothing the tab observes ever hears it. Its hover
    // tint watches the same node, rather than the whole tab.
    if let Some(close) = tab.child(|tab| tab.close()) {
        tab.ui.world.entity_mut(close).observe(
            move |_: On<Activate>, mut tree: ResMut<DockTree>| {
                tree.remove_tab(tab_id);
            },
        );

        tab.lit_entity(
            close,
            |tab| tab.close().icon().color(),
            close_hover,
            close_hover,
        );
    }
}

/// What a tab's text and icon are lit with: the active one reads
/// bright, the rest recede.
fn text_color(
    world: &World,
    leaf: NodeId,
    tab: TabId,
    primary: Color,
    muted: Color,
) -> Color {
    if active_of(world, leaf) == Some(tab) {
        primary
    } else {
        muted
    }
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
        column_gap: px(4),
        padding: UiRect::horizontal(px(8)),
        height: percent(100),
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
    color: Color,
) {
    let tile = world
        .spawn((
            tab_tile_node(),
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06)),
            ChildOf(wrapper),
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
        TextColor(color),
        ChildOf(tile),
    ));
    // Matches the 14px close slot a real tab reserves.
    world.spawn((
        Node {
            width: px(14),
            height: px(14),
            ..default()
        },
        ChildOf(tile),
    ));
}
