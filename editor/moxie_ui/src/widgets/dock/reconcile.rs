//! Materialize a [`DockTree`] into UI entities.
//!
//! Leaves become `DockArea`s (tab bar + content); splits become flex
//! containers wrapping two panels plus a [`SplitHandle`] between them.
//! Drag/move/resize mutate the tree only.
//!
//! The watcher fires on [`topology`], a fingerprint that omits split
//! fractions and the active tab — those change too often (a splitter
//! drag writes every frame) and ride on bindings instead. Only
//! structural edits rebuild the layout.

use std::fmt::Write as _;

use bevy::prelude::*;

use fynix_mock::elem;

use super::area::ActiveDockWindow;

use super::registry::{DockWindowBuildFn, WindowRegistry};
use super::tabs;
use super::tree::{
    DockAreaStyle, DockLeaf, DockNode, DockSplit, DockTree, NodeId,
    SplitAxis, TabId,
};
use crate::fynix::dock::{
    Area, AreaCursor, DockHost, SplitGroup, SplitHandle, SplitPanel,
    SplitPanelCursor, TabContent, TabContentCursor,
};
use crate::reactive::{BevyUi, resource_changed, structure_changed};

pub struct ReconcilePlugin;

impl Plugin for ReconcilePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DockTree>();
    }
}

/// The dock, as a node in the app's UI tree. Call this from the
/// builder handed to [`watch_root`](crate::reactive::watch_root).
pub fn dock(ui: &mut BevyUi) {
    super::add_popup::add_window_popup(ui);
    ui.elem(elem!(DockHost)).watch(
        structure_changed::<DockTree, _>(topology),
        build_dock,
    );
}

/// Marker for the node the dock tree is rendered underneath.
#[derive(Component, Clone, Debug, Default)]
pub struct DockTreeHost;

/// Binds an entity to a tree node. Present on both leaf-style
/// entities (`DockArea`) and split wrapper entities (`PanelGroup`).
#[derive(Component, Copy, Clone, Debug)]
pub struct NodeBinding(pub NodeId);

/// Alias kept for readability at call sites that only deal with
/// leaves.
pub type LeafBinding = NodeBinding;

/// Structural fingerprint of the tree: everything a rebuild depends
/// on. Split fractions and the active tab are excluded on purpose;
/// see the module docs.
fn topology(tree: &DockTree) -> String {
    let mut out = String::new();
    if let Some(root) = tree.root {
        write_topology(tree, root, &mut out);
    }
    out
}

fn write_topology(tree: &DockTree, id: NodeId, out: &mut String) {
    match tree.get(id) {
        Some(DockNode::Leaf(leaf)) => {
            let _ = write!(out, "L{id:?}:{}", leaf.area_id);
            for tab in &leaf.windows {
                let _ =
                    write!(out, "|{}#{:?}", tab.window_id, tab.id);
            }
            out.push(';');
        }
        Some(DockNode::Split(split)) => {
            let _ = write!(out, "S{:?}(", split.axis);
            write_topology(tree, split.a, out);
            out.push(',');
            write_topology(tree, split.b, out);
            out.push(')');
        }
        None => out.push('?'),
    }
}

fn build_dock(ui: &mut BevyUi) {
    let Some(root) = ui.world.resource::<DockTree>().root else {
        return;
    };
    build_node(root, ui);
}

fn build_node(id: NodeId, ui: &mut BevyUi) {
    let node = ui.world.resource::<DockTree>().get(id).cloned();
    match node {
        Some(DockNode::Leaf(leaf)) => build_leaf(id, leaf, ui),
        Some(DockNode::Split(split)) => build_split(id, split, ui),
        None => {}
    }
}

fn build_split(id: NodeId, split: DockSplit, ui: &mut BevyUi) {
    let flex_direction = match split.axis {
        SplitAxis::Horizontal => FlexDirection::Row,
        SplitAxis::Vertical => FlexDirection::Column,
    };

    // An empty, non-persistent leaf collapses so its sibling reclaims
    // the space. Derived here rather than patched afterwards: the
    // topology that decides it is the same one that triggered this
    // rebuild.
    let a_visible = leaf_visible(ui.world, split.a);
    let b_visible = leaf_visible(ui.world, split.b);
    let handle_visible = a_visible && b_visible;

    ui.elem(elem!(SplitGroup, node = id, axis = flex_direction))
        .with(move |ui| {
            build_panel(id, split.a, true, a_visible, ui);

            ui.elem(elem!(
                SplitHandle,
                node = id,
                visible = handle_visible
            ));

            build_panel(id, split.b, false, b_visible, ui);
        });
}

/// One side of a split. The ratio is a binding, not part of the
/// build: dragging the handle rewrites the fraction every frame and
/// must not rebuild anything.
fn build_panel(
    split_id: NodeId,
    child: NodeId,
    is_a: bool,
    visible: bool,
    ui: &mut BevyUi,
) {
    let ratio = ratio_of(ui.world, split_id, is_a);

    ui.elem(elem!(SplitPanel, ratio = ratio, visible = visible))
        .bind(
            |panel| panel.ratio(),
            resource_changed::<DockTree>(),
            move |world, _| ratio_of(world, split_id, is_a),
        )
        .with(move |ui| build_node(child, ui));
}

/// This side's share of its split.
fn ratio_of(world: &World, split_id: NodeId, is_a: bool) -> f32 {
    match world.resource::<DockTree>().get(split_id) {
        Some(DockNode::Split(split)) if is_a => split.fraction,
        Some(DockNode::Split(split)) => 1.0 - split.fraction,
        _ => 1.0,
    }
}

/// Persistent leaves stay visible when empty so they remain drop
/// targets; other empty leaves collapse.
fn leaf_visible(world: &World, id: NodeId) -> bool {
    match world.resource::<DockTree>().get(id) {
        Some(DockNode::Leaf(leaf)) => {
            !leaf.windows.is_empty() || leaf.is_persistent()
        }
        _ => true,
    }
}

fn build_leaf(id: NodeId, leaf: DockLeaf, ui: &mut BevyUi) {
    // Resolve tabs against the registry once, cloning out: `ui` takes
    // the world mutably from here on. Iterating `leaf.windows` (not
    // the registry) keeps two tabs of the same window kind distinct.
    let registry = ui.world.resource::<WindowRegistry>();
    let tabs_data = leaf
        .windows
        .iter()
        .filter_map(|tab| {
            let desc = registry.get(&tab.window_id)?;
            Some((
                tab.id,
                desc.id.clone(),
                desc.name.clone(),
                desc.icon.clone(),
                desc.build.clone(),
            ))
        })
        .collect::<Vec<_>>();

    let bar_tabs = tabs_data
        .iter()
        .map(|(tab_id, window_id, name, icon, _)| {
            (*tab_id, window_id.clone(), name.clone(), icon.clone())
        })
        .collect::<Vec<_>>();

    let area_id = leaf.area_id.clone();
    let style = leaf.style.clone();
    let show_bar = matches!(leaf.style, DockAreaStyle::TabBar);
    let active = active_of(ui.world, id);

    let area = ui
        .elem(elem!(
            Area,
            node = id,
            id = area_id,
            style = style,
            active = ActiveDockWindow(active)
        ))
        .bind(
            |area| area.active(),
            resource_changed::<DockTree>(),
            move |world, _| ActiveDockWindow(active_of(world, id)),
        );

    // The area's handle is in hand, so the tab bar gets it directly
    // rather than walking up for it.
    let area_entity = area.id();
    area.with(move |ui| {
        // Bar first, then content: the bar must be a direct child of
        // the area for drag hit-testing.
        if show_bar {
            tabs::build_tab_bar(id, area_entity, bar_tabs, ui);
        }
        for (tab_id, window_id, _, _, build) in tabs_data {
            build_content(id, tab_id, window_id, build, ui);
        }
    });
}

/// A tab's content pane. Switching tabs flips `Display` through a
/// binding rather than rebuilding: the content owns cameras, scroll
/// offsets and live edits that have to survive a tab switch.
fn build_content(
    leaf: NodeId,
    tab: TabId,
    window_id: String,
    build: DockWindowBuildFn,
    ui: &mut BevyUi,
) {
    let active = active_of(ui.world, leaf) == Some(tab);

    ui.elem(elem!(
        TabContent,
        window_id = window_id,
        tab = tab,
        showing = active
    ))
    .bind(
        |content| content.showing(),
        resource_changed::<DockTree>(),
        move |world, _| active_of(world, leaf) == Some(tab),
    )
    // The window's own content, as elements of its own.
    .with(move |ui| build(ui));
}

fn active_of(world: &World, id: NodeId) -> Option<TabId> {
    match world.resource::<DockTree>().get(id) {
        Some(DockNode::Leaf(leaf)) => leaf.active,
        _ => None,
    }
}
