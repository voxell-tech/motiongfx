//! Materialize a [`DockTree`] into UI entities.
//!
//! The tree is the source of truth for layout. Editor code spawns one
//! `DockTreeHost` entity, and the reconciler walks the tree from its
//! single root and shapes the entity sub-tree to match: leaves become
//! `DockArea`s with tab bar + content, splits become flex containers
//! wrapping two child panel entities plus a `PanelHandle` between
//! them.
//!
//! Drag/move/resize operations mutate the tree only; the reconciler
//! rebuilds the affected entity sub-tree on the next frame.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use super::area::{
    ActiveDockWindow, DockArea, DockTabContent, DockWindow,
};
use super::registry::WindowRegistry;
use super::split::{Panel, PanelGroup, PanelHandle};
use super::tabs;
use super::tree::{
    DockAreaStyle, DockLeaf, DockNode, DockSplit, DockTree, NodeId,
    SplitAxis,
};
use crate::glass::Glass;

pub struct ReconcilePlugin;

impl Plugin for ReconcilePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DockTree>().add_systems(
            Update,
            (reconcile_tree, sync_leaf_visuals).chain(),
        );
    }
}

/// Marker for the single host entity the reconciler renders the dock
/// tree underneath. Spawn one of these (typically the editor's main
/// content area); the reconciler fills it with split / leaf entities
/// matching the current [`DockTree`].
#[derive(Component, Clone, Debug, Default)]
pub struct DockTreeHost;

/// Binds an entity to a tree node. Present on both leaf-style
/// entities (`DockArea`) and split wrapper entities (`PanelGroup`).
#[derive(Component, Copy, Clone, Debug)]
pub struct NodeBinding(pub NodeId);

/// Alias kept for readability at call sites that only deal with
/// leaves.
pub type LeafBinding = NodeBinding;

fn reconcile_tree(world: &mut World) {
    if !world.is_resource_changed::<DockTree>() {
        return;
    }
    let Some(root) = world.resource::<DockTree>().root else {
        return;
    };
    let Some(host) = find_dock_tree_host(world) else {
        return;
    };
    reconcile_at(world, host, root);
}

fn find_dock_tree_host(world: &mut World) -> Option<Entity> {
    let mut q = world.query::<(Entity, &DockTreeHost)>();
    q.iter(world).next().map(|(e, _)| e)
}

fn reconcile_at(world: &mut World, entity: Entity, node_id: NodeId) {
    let node = world.resource::<DockTree>().get(node_id).cloned();
    let Some(node) = node else {
        return;
    };
    match node {
        DockNode::Leaf(leaf) => {
            reconcile_leaf(world, entity, node_id, &leaf)
        }
        DockNode::Split(split) => {
            reconcile_split(world, entity, node_id, &split)
        }
    }
}

fn reconcile_leaf(
    world: &mut World,
    entity: Entity,
    node_id: NodeId,
    leaf: &DockLeaf,
) {
    let current_binding =
        world.entity(entity).get::<NodeBinding>().map(|b| b.0);
    let was_split = world.entity(entity).contains::<PanelGroup>();
    let current_tabs = collect_content_tab_ids(world, entity);
    let leaf_tabs: Vec<super::tree::TabId> =
        leaf.windows.iter().map(|t| t.id).collect();

    let needs_rebuild = was_split
        || current_binding != Some(node_id)
        || current_tabs != leaf_tabs;

    if needs_rebuild {
        despawn_children(world, entity);
        world.entity_mut(entity).remove::<PanelGroup>();

        let direction = match leaf.style {
            DockAreaStyle::TabBar | DockAreaStyle::Headless => {
                FlexDirection::Column
            }
        };
        if let Some(mut node) =
            world.entity_mut(entity).get_mut::<Node>()
        {
            node.flex_direction = direction;
        }

        if let Some(mut area) =
            world.entity_mut(entity).get_mut::<DockArea>()
        {
            area.id = leaf.area_id.clone();
            area.style = leaf.style.clone();
        } else {
            world.entity_mut(entity).insert(DockArea {
                id: leaf.area_id.clone(),
                style: leaf.style.clone(),
            });
        }

        spawn_leaf_ui(world, entity, leaf);
    }

    world
        .entity_mut(entity)
        .insert(ActiveDockWindow(leaf.active));
    world.entity_mut(entity).insert(NodeBinding(node_id));

    // Auto-collapse: when a non-persistent leaf has no windows, hide
    // the host entity and its adjacent handle so siblings can reclaim
    // the space. Persistent leaves stay visible even when empty so
    // they remain drop targets.
    let visible = !leaf.windows.is_empty() || leaf.is_persistent();
    set_host_visible(world, entity, visible);
}

fn reconcile_split(
    world: &mut World,
    entity: Entity,
    node_id: NodeId,
    split: &DockSplit,
) {
    let current_binding =
        world.entity(entity).get::<NodeBinding>().map(|b| b.0);

    let mut children = collect_split_children(world, entity);
    let needs_rebuild =
        current_binding != Some(node_id) || children.is_none();

    if needs_rebuild {
        despawn_children(world, entity);
        world.entity_mut(entity).remove::<ActiveDockWindow>();
        world.entity_mut(entity).remove::<DockArea>();

        if let Some(mut node) =
            world.entity_mut(entity).get_mut::<Node>()
        {
            node.flex_direction = match split.axis {
                SplitAxis::Horizontal => FlexDirection::Row,
                SplitAxis::Vertical => FlexDirection::Column,
            };
        }
        if !world.entity(entity).contains::<PanelGroup>() {
            world
                .entity_mut(entity)
                .insert(PanelGroup { min_ratio: 0.05 });
        }

        let child_node = || Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip(),
            ..default()
        };

        let child_a = world
            .spawn((
                Panel {
                    ratio: split.fraction,
                },
                child_node(),
                ChildOf(entity),
            ))
            .id();
        let handle = world
            .spawn((
                PanelHandle,
                Node {
                    min_width: Val::Px(3.0),
                    min_height: Val::Px(3.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                NodeBinding(node_id),
                ChildOf(entity),
            ))
            .id();
        let child_b = world
            .spawn((
                Panel {
                    ratio: 1.0 - split.fraction,
                },
                child_node(),
                ChildOf(entity),
            ))
            .id();
        children = Some((child_a, handle, child_b));
    }

    let (child_a, _handle, child_b) =
        children.expect("children exist after rebuild");

    if let Some(mut p) = world.entity_mut(child_a).get_mut::<Panel>()
        && (p.ratio - split.fraction).abs() > f32::EPSILON
    {
        p.ratio = split.fraction;
    }
    if let Some(mut p) = world.entity_mut(child_b).get_mut::<Panel>()
    {
        let other = 1.0 - split.fraction;
        if (p.ratio - other).abs() > f32::EPSILON {
            p.ratio = other;
        }
    }

    reconcile_at(world, child_a, split.a);
    reconcile_at(world, child_b, split.b);

    world.entity_mut(entity).insert(NodeBinding(node_id));

    // A split always has visible leaf children, so the container must
    // be visible. If the host was collapsed just before the
    // transition, restore it here so the freshly-reconciled children
    // aren't hidden inside a zero-sized parent.
    set_host_visible(world, entity, true);
}

fn spawn_leaf_ui(world: &mut World, entity: Entity, leaf: &DockLeaf) {
    // Iterate `leaf.windows` so two tabs of the same window kind
    // produce two entries with distinct `TabId`s.
    let snapshot: Vec<(
        super::tree::TabId,
        String,
        String,
        super::registry::DockWindowBuildFn,
    )> = {
        let registry = world.resource::<WindowRegistry>();
        leaf.windows
            .iter()
            .filter_map(|tab| {
                let desc = registry.get(&tab.window_id)?;
                Some((
                    tab.id,
                    desc.id.clone(),
                    desc.name.clone(),
                    desc.build.clone(),
                ))
            })
            .collect()
    };

    match leaf.style {
        DockAreaStyle::TabBar => {
            let tabs_data: Vec<(super::tree::TabId, String, String)> =
                snapshot
                    .iter()
                    .map(|(tab_id, id, name, _)| {
                        (*tab_id, id.clone(), name.clone())
                    })
                    .collect();
            tabs::spawn_tab_bar_world(
                world,
                entity,
                &tabs_data,
                leaf.active,
            );
        }
        DockAreaStyle::Headless => {}
    }

    for (tab_id, window_id, _name, build) in &snapshot {
        let is_active = leaf.active == Some(*tab_id);
        let content_entity = world
            .spawn((
                DockWindow {
                    descriptor_id: window_id.clone(),
                    tab_id: *tab_id,
                },
                DockTabContent {
                    window_id: window_id.clone(),
                    tab_id: *tab_id,
                },
                Node {
                    flex_grow: 1.0,
                    width: Val::Percent(100.0),
                    min_height: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip(),
                    display: if is_active {
                        Display::Flex
                    } else {
                        Display::None
                    },
                    ..default()
                },
                ChildOf(entity),
            ))
            .id();
        (build)(&mut ChildSpawner::new(world, content_entity));
    }
}

fn collect_content_tab_ids(
    world: &mut World,
    entity: Entity,
) -> Vec<super::tree::TabId> {
    let children: Vec<Entity> = world
        .entity(entity)
        .get::<Children>()
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    let mut out = Vec::new();
    for child in children {
        if let Some(c) = world.entity(child).get::<DockTabContent>() {
            out.push(c.tab_id);
        }
    }
    out
}

/// If `entity` currently looks like a split host (`PanelGroup` with
/// three children: panel, handle, panel), return them in order.
fn collect_split_children(
    world: &mut World,
    entity: Entity,
) -> Option<(Entity, Entity, Entity)> {
    let children: Vec<Entity> = world
        .entity(entity)
        .get::<Children>()
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    if children.len() != 3 {
        return None;
    }
    let a = children[0];
    let h = children[1];
    let b = children[2];
    if !world.entity(h).contains::<PanelHandle>() {
        return None;
    }
    if !world.entity(a).contains::<Panel>()
        || !world.entity(b).contains::<Panel>()
    {
        return None;
    }
    Some((a, h, b))
}

/// Show or hide a host entity and its adjacent `PanelHandle` sibling
/// so an empty leaf doesn't leave a stub panel + dangling resize
/// handle.
fn set_host_visible(
    world: &mut World,
    entity: Entity,
    visible: bool,
) {
    let target = if visible {
        Display::Flex
    } else {
        Display::None
    };

    // Find the adjacent PanelHandle sibling (index +/-1 in the
    // parent's children) so we can hide/show it alongside the
    // host.
    let adjacent_handle = {
        let parent = world
            .entity(entity)
            .get::<ChildOf>()
            .map(ChildOf::parent);
        parent.and_then(|parent| {
            let siblings: Vec<Entity> = world
                .entity(parent)
                .get::<Children>()
                .map(|c| c.iter().collect())
                .unwrap_or_default();
            let idx = siblings.iter().position(|&e| e == entity)?;
            [idx.checked_sub(1), Some(idx + 1)]
                .into_iter()
                .flatten()
                .filter_map(|i| siblings.get(i).copied())
                .find(|&e| world.entity(e).contains::<PanelHandle>())
        })
    };

    let mut any_changed = false;

    // Host: toggle Display and drive geometry only when the state
    // actually transitions. Unconditionally setting width/height
    // every reconcile pass would stomp on the ratio-based
    // percentages that `recalculate_group` has already written
    // for an already-visible panel, producing a panel that fills
    // 100% of its Row parent.
    if let Some(mut node) = world.entity_mut(entity).get_mut::<Node>()
    {
        if node.display != target {
            node.display = target;
            any_changed = true;
        }
        let zero = Val::Px(0.0);
        if !visible {
            if node.width != zero || node.height != zero {
                node.width = zero;
                node.height = zero;
                node.min_width = zero;
                node.min_height = zero;
                any_changed = true;
            }
        } else if node.width == zero {
            node.width = Val::Percent(100.0);
            node.height = Val::Percent(100.0);
            any_changed = true;
        }
    }

    // Handle: ONLY toggle Display. Don't touch width/height. A
    // `PanelHandle`'s natural size is a 3px stripe along the flex
    // axis; forcing 100% would make it fill the parent.
    if let Some(handle) = adjacent_handle
        && let Some(mut node) =
            world.entity_mut(handle).get_mut::<Node>()
        && node.display != target
    {
        node.display = target;
        any_changed = true;
    }

    // Flag the host's Panel as changed so `recalculate_group`
    // redistributes sibling widths this frame.
    if any_changed
        && let Some(mut panel) =
            world.entity_mut(entity).get_mut::<Panel>()
    {
        panel.set_changed();
    }
}

fn despawn_children(world: &mut World, entity: Entity) {
    let children: Vec<Entity> = world
        .entity(entity)
        .get::<Children>()
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    for child in children {
        if let Ok(em) = world.get_entity_mut(child) {
            em.despawn();
        }
    }
}

/// On every Update, sync visual state (active tab bg/border/text
/// colors, content `Display`) for leaf entities.
fn sync_leaf_visuals(
    leaves: Query<
        (Entity, &NodeBinding, &DockArea),
        Without<PanelGroup>,
    >,
    tree: Res<DockTree>,
    tabs: Query<(Entity, &super::area::DockTab, &ChildOf)>,
    contents: Query<(Entity, &DockTabContent, &ChildOf)>,
    parent_query: Query<&ChildOf>,
    children_query: Query<&Children>,
    mut nodes: Query<&mut Node>,
    mut text_colors: Query<&mut TextColor>,
    theme: Res<crate::theme::EditorTheme>,
    mut commands: Commands,
) {
    if !tree.is_changed() {
        return;
    }

    let mut tab_to_area: HashMap<Entity, Entity> = HashMap::new();
    for (tab_entity, _, child_of) in &tabs {
        let tab_row = child_of.parent();
        let Ok(row_parent) = parent_query.get(tab_row) else {
            continue;
        };
        let tab_bar = row_parent.parent();
        let Ok(bar_parent) = parent_query.get(tab_bar) else {
            continue;
        };
        tab_to_area.insert(tab_entity, bar_parent.parent());
    }

    for (area_entity, binding, _) in &leaves {
        let Some(leaf) =
            tree.get(binding.0).and_then(|n| n.as_leaf())
        else {
            continue;
        };

        for (tab_entity, tab, _) in &tabs {
            if tab_to_area.get(&tab_entity) != Some(&area_entity) {
                continue;
            }
            let is_active = leaf.active == Some(tab.tab_id);
            // Re-inserting the preset swaps the tab's material.
            commands.entity(tab_entity).insert(Glass::tab(is_active));
            if let Ok(tab_children) = children_query.get(tab_entity) {
                for child in tab_children.iter() {
                    if let Ok(mut tc) = text_colors.get_mut(child) {
                        tc.0 = if is_active {
                            theme.text_primary
                        } else {
                            theme.text_muted
                        };
                    }
                }
            }
        }

        for (content_entity, content, child_of) in &contents {
            if child_of.parent() != area_entity {
                continue;
            }
            let should_show = leaf.active == Some(content.tab_id);
            let target = if should_show {
                Display::Flex
            } else {
                Display::None
            };
            if let Ok(mut node) = nodes.get_mut(content_entity)
                && node.display != target
            {
                node.display = target;
            }
        }
    }
}
