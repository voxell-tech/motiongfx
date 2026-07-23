//! Drag/drop state machine: drag a tab to reorder it within a tab
//! bar, merge it into another leaf, or split an area on drop.

use bevy::feathers::cursor::{EntityCursor, OverrideCursor};
use bevy::prelude::*;
use bevy::ui::{UiGlobalTransform, UiScale};
use bevy::window::SystemCursorIcon;

use super::area::DockArea;
use super::reconcile::NodeBinding;
use super::registry::WindowRegistry;
use super::tabs::DockTabRow;
use super::tree::{DockTree, Edge as TreeEdge, TabId};
use crate::glass::Glass;

pub struct DockDragPlugin;

impl Plugin for DockDragPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DockDragState>()
            .add_observer(on_tab_drag_start)
            .add_observer(on_drag_move)
            .add_observer(on_drag_end)
            .add_systems(Update, cancel_drag_on_escape);
    }
}

const DRAG_THRESHOLD: f32 = 5.0;

#[derive(Resource, Default, Debug)]
pub enum DockDragState {
    #[default]
    Idle,
    PendingDrag {
        source_tab: Entity,
        tab_id: TabId,
        window_id: String,
        window_name: String,
        start_pos: Vec2,
    },
    Dragging {
        source_tab: Entity,
        tab_id: TabId,
        window_id: String,
        window_name: String,
        source_area: Entity,
        ghost_entity: Entity,
        cursor_pos: Vec2,
        drop_target: Option<DropTarget>,
        overlay_entity: Option<Entity>,
    },
}

#[derive(Clone, Debug)]
pub enum DropTarget {
    Panel(Entity),
    TabRow { bar: Entity, index: usize },
    AreaEdge { area: Entity, edge: DropEdge },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropEdge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Component)]
pub struct DragGhost;

#[derive(Component)]
pub struct DropOverlay;

/// Node rect in logical (UI) coordinates. Shared with the add-popup.
pub(super) fn logical_rect(
    computed: &ComputedNode,
    transform: &UiGlobalTransform,
) -> Rect {
    let inv = computed.inverse_scale_factor();
    let size = computed.size() * inv;
    let (_scale, _angle, center) =
        transform.to_scale_angle_translation();
    let center = center.trunc() * inv;
    Rect::from_center_size(center, size)
}

fn on_tab_drag_start(
    trigger: On<Pointer<DragStart>>,
    tabs: Query<&super::area::DockTab>,
    mut drag_state: ResMut<DockDragState>,
    registry: Res<WindowRegistry>,
    ui_scale: Res<UiScale>,
) {
    let entity = trigger.event_target();
    let Ok(tab) = tabs.get(entity) else { return };

    let display_name = registry
        .get(&tab.window_id)
        .map(|d| d.name.clone())
        .unwrap_or_else(|| tab.window_id.clone());

    *drag_state = DockDragState::PendingDrag {
        source_tab: entity,
        tab_id: tab.tab_id,
        window_id: tab.window_id.clone(),
        window_name: display_name,
        start_pos: Vec2::new(
            trigger.event().pointer_location.position.x,
            trigger.event().pointer_location.position.y,
        ) / ui_scale.0,
    };
}

fn on_drag_move(
    mut trigger: On<Pointer<Drag>>,
    mut drag_state: ResMut<DockDragState>,
    mut commands: Commands,
    areas: Query<
        (Entity, &ComputedNode, &UiGlobalTransform),
        With<DockArea>,
    >,
    tab_rows: Query<
        (
            Entity,
            &ComputedNode,
            &Node,
            &UiGlobalTransform,
            &Children,
            &ChildOf,
        ),
        With<DockTabRow>,
    >,
    node_query: Query<(&ComputedNode, &UiGlobalTransform)>,
    parent_query: Query<&ChildOf>,
    ui_scale: Res<UiScale>,
    mut override_cursor: ResMut<OverrideCursor>,
) {
    let drag_event = trigger.event();
    let cursor_pos_ui = Vec2::new(
        drag_event.pointer_location.position.x,
        drag_event.pointer_location.position.y,
    ) / ui_scale.0;

    match &*drag_state {
        DockDragState::PendingDrag {
            source_tab,
            tab_id,
            window_id,
            window_name,
            start_pos,
        } => {
            if cursor_pos_ui.distance(*start_pos) < DRAG_THRESHOLD {
                return;
            }

            let source_tab = *source_tab;
            let tab_id = *tab_id;
            let window_id = window_id.clone();
            let window_name = window_name.clone();

            let source_area =
                find_parent_area(source_tab, &parent_query, &areas);

            // Drag underway: show the grabbing cursor everywhere.
            if override_cursor.is_none() {
                override_cursor.0 = Some(EntityCursor::System(
                    SystemCursorIcon::Grabbing,
                ));
            }

            // The ghost is the real tab tile, rebuilt via the shared
            // builder so it's identical; hide the original meanwhile.
            let ghost = commands
                .spawn((
                    DragGhost,
                    ghost_node(cursor_pos_ui),
                    GlobalZIndex(200),
                ))
                .id();
            commands.entity(source_tab).insert(Visibility::Hidden);
            let name = window_name.clone();
            commands.queue(move |world: &mut World| {
                super::tabs::spawn_ghost_tab(world, ghost, &name);
            });

            *drag_state = DockDragState::Dragging {
                source_tab,
                tab_id,
                window_id,
                window_name,
                source_area: source_area
                    .unwrap_or(Entity::PLACEHOLDER),
                ghost_entity: ghost,
                cursor_pos: cursor_pos_ui,
                drop_target: None,
                overlay_entity: None,
            };

            trigger.propagate(false);
        }
        DockDragState::Dragging {
            ghost_entity,
            overlay_entity,
            ..
        } => {
            let ghost = *ghost_entity;
            let old_overlay = *overlay_entity;

            commands.entity(ghost).insert(ghost_node(cursor_pos_ui));

            if let Some(old) = old_overlay {
                commands.entity(old).despawn();
            }

            let mut new_target = None;
            let mut new_overlay = None;

            for (
                tab_row_entity,
                computed,
                node,
                ui_transform,
                children,
                parent,
            ) in &tab_rows
            {
                let row_rect = logical_rect(computed, ui_transform);
                let parent_contains =
                    node_query.get(parent.0).is_ok_and(
                        |(parent_computed, parent_transform)| {
                            logical_rect(
                                parent_computed,
                                parent_transform,
                            )
                            .contains(cursor_pos_ui)
                        },
                    );
                if !row_rect.contains(cursor_pos_ui)
                    && !parent_contains
                {
                    continue;
                }
                let mut closest_child: Option<(
                    Vec2,
                    Vec2,
                    usize,
                    f32,
                )> = None;
                for (index, child) in children.iter().enumerate() {
                    let Ok((child_computed, child_transform)) =
                        node_query.get(child)
                    else {
                        continue;
                    };
                    let child_rect =
                        logical_rect(child_computed, child_transform);
                    let child_center = child_rect.center();
                    let child_size = child_rect.size();
                    let distance =
                        child_center.distance_squared(cursor_pos_ui);
                    if closest_child.is_none_or(
                        |(_, _, _, closest_dist)| {
                            distance < closest_dist
                        },
                    ) {
                        closest_child = Some((
                            child_center,
                            child_size,
                            index,
                            distance,
                        ));
                    }
                }
                let Some((child_center, child_size, mut index, _)) =
                    closest_child
                else {
                    continue;
                };
                let (is_far_side, is_vertical) =
                    is_far_side(cursor_pos_ui, child_center, node);
                if is_far_side {
                    index += 1;
                }

                new_target = Some(DropTarget::TabRow {
                    bar: tab_row_entity,
                    index,
                });

                let size_mult = if !is_vertical {
                    Vec2::new(0.5, 1.0)
                } else {
                    Vec2::new(1.0, 0.5)
                };

                let overlay_size = child_size * size_mult;

                let mut offset = if !is_vertical {
                    Vec2::new(child_size.x, overlay_size.y)
                } else {
                    Vec2::new(overlay_size.x, child_size.y)
                };

                offset *= -0.5;

                if is_far_side {
                    if !is_vertical {
                        offset.x = 0.0;
                    } else {
                        offset.y = 0.0;
                    }
                }
                let overlay_pos = child_center + offset;

                let overlay = commands
                    .spawn((
                        DropOverlay,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(overlay_pos.x),
                            top: Val::Px(overlay_pos.y),
                            width: Val::Px(overlay_size.x),
                            height: Val::Px(overlay_size.y),
                            border_radius: BorderRadius::all(
                                Val::Px(4.0),
                            ),
                            ..Default::default()
                        },
                        Glass::Overlay,
                        GlobalZIndex(150),
                    ))
                    .id();

                new_overlay = Some(overlay);
                break;
            }

            if new_target.is_none() {
                for (area_entity, computed, ui_transform) in &areas {
                    let area_rect =
                        logical_rect(computed, ui_transform);
                    if !area_rect.contains(cursor_pos_ui) {
                        continue;
                    }

                    if let Some(edge) =
                        cursor_edge(area_rect, cursor_pos_ui)
                    {
                        new_target = Some(DropTarget::AreaEdge {
                            area: area_entity,
                            edge,
                        });

                        let overlay_rect =
                            edge_overlay_rect(area_rect, edge);
                        let overlay = commands
                            .spawn((
                                DropOverlay,
                                Node {
                                    position_type:
                                        PositionType::Absolute,
                                    left: Val::Px(overlay_rect.min.x),
                                    top: Val::Px(overlay_rect.min.y),
                                    width: Val::Px(
                                        overlay_rect.size().x,
                                    ),
                                    height: Val::Px(
                                        overlay_rect.size().y,
                                    ),
                                    border_radius: BorderRadius::all(
                                        Val::Px(4.0),
                                    ),
                                    ..default()
                                },
                                Glass::Overlay,
                                GlobalZIndex(150),
                            ))
                            .id();
                        new_overlay = Some(overlay);
                    } else {
                        new_target =
                            Some(DropTarget::Panel(area_entity));

                        let overlay = commands
                            .spawn((
                                DropOverlay,
                                Node {
                                    position_type:
                                        PositionType::Absolute,
                                    left: Val::Px(area_rect.min.x),
                                    top: Val::Px(area_rect.min.y),
                                    width: Val::Px(
                                        area_rect.size().x,
                                    ),
                                    height: Val::Px(
                                        area_rect.size().y,
                                    ),
                                    border_radius: BorderRadius::all(
                                        Val::Px(4.0),
                                    ),
                                    ..default()
                                },
                                Glass::Overlay,
                                GlobalZIndex(150),
                            ))
                            .id();
                        new_overlay = Some(overlay);
                    }

                    break;
                }
            }

            if let DockDragState::Dragging {
                drop_target,
                overlay_entity,
                cursor_pos,
                ..
            } = &mut *drag_state
            {
                *drop_target = new_target;
                *overlay_entity = new_overlay;
                *cursor_pos = cursor_pos_ui;
            }

            trigger.propagate(false);
        }
        _ => {}
    }
}

fn on_drag_end(
    _trigger: On<Pointer<DragEnd>>,
    mut drag_state: ResMut<DockDragState>,
    mut override_cursor: ResMut<OverrideCursor>,
    mut commands: Commands,
) {
    let state = std::mem::take(&mut *drag_state);
    match state {
        DockDragState::Dragging {
            source_tab,
            ghost_entity,
            overlay_entity,
            drop_target,
            tab_id,
            source_area,
            ..
        } => {
            clear_grab_cursor(&mut override_cursor);
            commands.entity(ghost_entity).despawn();
            // Reveal the original tab again (a consumed drop rebuilds
            // the leaf and despawns it anyway, so this is
            // best-effort).
            commands
                .entity(source_tab)
                .try_insert(Visibility::Inherited);
            if let Some(overlay) = overlay_entity {
                commands.entity(overlay).despawn();
            }

            if let Some(target) = drop_target {
                match target {
                    DropTarget::Panel(target_area) => {
                        if target_area != source_area {
                            commands.queue(
                                move |world: &mut World| {
                                    drop_on_area(
                                        world,
                                        tab_id,
                                        target_area,
                                    );
                                },
                            );
                        }
                    }
                    DropTarget::AreaEdge { area, edge } => {
                        commands.queue(move |world: &mut World| {
                            drop_on_edge(world, tab_id, area, edge);
                        });
                    }
                    DropTarget::TabRow { bar, index } => {
                        commands.queue(move |world: &mut World| {
                            drop_on_tab_row(
                                world, tab_id, bar, index,
                            );
                        });
                    }
                }
            }
        }
        DockDragState::PendingDrag { .. } | DockDragState::Idle => {}
    }

    *drag_state = DockDragState::Idle;
}

fn cancel_drag_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut drag_state: ResMut<DockDragState>,
    mut override_cursor: ResMut<OverrideCursor>,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    let state = std::mem::take(&mut *drag_state);
    if let DockDragState::Dragging {
        source_tab,
        ghost_entity,
        overlay_entity,
        ..
    } = state
    {
        clear_grab_cursor(&mut override_cursor);
        commands.entity(ghost_entity).despawn();
        commands
            .entity(source_tab)
            .try_insert(Visibility::Inherited);
        if let Some(overlay) = overlay_entity {
            commands.entity(overlay).despawn();
        }
    }

    *drag_state = DockDragState::Idle;
}

/// The drag-ghost wrapper node at `cursor` (logical px); its child is
/// the reused tab tile, so it only carries position + height.
fn ghost_node(cursor: Vec2) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(cursor.x - 40.0),
        top: Val::Px(cursor.y - 12.0),
        height: Val::Px(super::TAB_HEIGHT),
        ..default()
    }
}

/// Drop the drag-wide grabbing cursor, if it's the one we set.
fn clear_grab_cursor(override_cursor: &mut OverrideCursor) {
    if override_cursor.0
        == Some(EntityCursor::System(SystemCursorIcon::Grabbing))
    {
        override_cursor.0 = None;
    }
}

/// Move the dragged tab into the leaf bound to `target_area`.
fn drop_on_area(world: &mut World, tab: TabId, target_area: Entity) {
    let Some(binding) =
        world.entity(target_area).get::<NodeBinding>().copied()
    else {
        return;
    };
    world.resource_mut::<DockTree>().move_tab(tab, binding.0);
}

/// Split the leaf bound to `target_area` along `edge` and reseat the
/// dragged tab into the new sibling. The tab keeps its window kind
/// but receives a fresh [`TabId`] (we remove + split rather than
/// move, since `tree.split` builds the leaf from a window id).
fn drop_on_edge(
    world: &mut World,
    tab: TabId,
    target_area: Entity,
    edge: DropEdge,
) {
    let Some(binding) =
        world.entity(target_area).get::<NodeBinding>().copied()
    else {
        return;
    };
    let tree_edge = match edge {
        DropEdge::Top => TreeEdge::Top,
        DropEdge::Bottom => TreeEdge::Bottom,
        DropEdge::Left => TreeEdge::Left,
        DropEdge::Right => TreeEdge::Right,
    };
    let mut tree = world.resource_mut::<DockTree>();
    let Some(window_id) =
        tree.find_leaf_for_tab(tab).and_then(|leaf_id| {
            tree.get(leaf_id)
                .and_then(|n| n.as_leaf())
                .and_then(|l| l.windows.iter().find(|t| t.id == tab))
                .map(|t| t.window_id.clone())
        })
    else {
        return;
    };
    // Split first (keeps the target leaf valid), then remove: the
    // reverse can collapse the target when it's the tab's last leaf.
    if tree.split(binding.0, tree_edge, window_id).is_some() {
        tree.remove_tab(tab);
    }
}

/// Drop the dragged tab onto the leaf bound to `tab_row` at slot
/// `index`. Reordering within the source leaf is allowed (drag a tab
/// to reorder it).
fn drop_on_tab_row(
    world: &mut World,
    tab: TabId,
    tab_row: Entity,
    index: usize,
) {
    let mut parent_query = world.query::<&ChildOf>();
    let parent_query = parent_query.query(world);

    let mut binding = None;
    for parent in parent_query.iter_ancestors(tab_row) {
        if let Some(node_binding) =
            world.entity(parent).get::<NodeBinding>()
        {
            binding = Some(node_binding);
            break;
        }
    }

    let Some(binding) = binding.copied() else {
        warn!(
            "No `NodeBinding` found in parents of tab row {tab_row}"
        );
        return;
    };

    let mut tree = world.resource_mut::<DockTree>();
    tree.insert_tab(tab, binding.0, true, Some(index));
}

fn find_parent_area(
    entity: Entity,
    parents: &Query<&ChildOf>,
    areas: &Query<
        (Entity, &ComputedNode, &UiGlobalTransform),
        With<DockArea>,
    >,
) -> Option<Entity> {
    let mut current = entity;
    loop {
        if areas.contains(current) {
            return Some(current);
        }
        let Ok(parent) = parents.get(current) else {
            return None;
        };
        current = parent.parent();
    }
}

fn cursor_edge(rect: Rect, cursor: Vec2) -> Option<DropEdge> {
    let rel = cursor - rect.center();
    let frac_x = rel.x / rect.size().x;
    let frac_y = rel.y / rect.size().y;

    // The center region is a no-op. The outer n% on each side are the
    // drop edges. All four edges are equal: dropping on the top of
    // any panel splits it vertically with the dragged window
    // above.
    const EDGE_PERCENT: f32 = 0.25;

    if frac_x < -EDGE_PERCENT {
        Some(DropEdge::Left)
    } else if frac_x > EDGE_PERCENT {
        Some(DropEdge::Right)
    } else if frac_y > EDGE_PERCENT {
        Some(DropEdge::Bottom)
    } else if frac_y < -EDGE_PERCENT {
        Some(DropEdge::Top)
    } else {
        None
    }
}

fn edge_overlay_rect(rect: Rect, edge: DropEdge) -> Rect {
    let (axis, factor) = match edge {
        DropEdge::Top => {
            (-Vec2::Y * rect.size().y, Vec2::new(1.0, 0.5))
        }
        DropEdge::Bottom => {
            (Vec2::Y * rect.size().y, Vec2::new(1.0, 0.5))
        }
        DropEdge::Left => {
            (-Vec2::X * rect.size().x, Vec2::new(0.5, 1.0))
        }
        DropEdge::Right => {
            (Vec2::X * rect.size().x, Vec2::new(0.5, 1.0))
        }
    };
    // Half the axis length shifts the center by 25% of that axis so
    // the overlay covers exactly half of the area along a given
    // axis.
    Rect::from_center_size(
        rect.center() + axis * 0.25,
        rect.size() * factor,
    )
}

fn is_far_side(
    mouse_pos: Vec2,
    child_pos: Vec2,
    parent: &Node,
) -> (bool, bool) {
    return match parent.flex_direction {
        FlexDirection::Row => {
            (is_far_side(mouse_pos, child_pos, false), false)
        }
        FlexDirection::RowReverse => {
            (!is_far_side(mouse_pos, child_pos, false), false)
        }
        FlexDirection::Column => {
            (is_far_side(mouse_pos, child_pos, true), true)
        }
        FlexDirection::ColumnReverse => {
            (!is_far_side(mouse_pos, child_pos, true), true)
        }
    };

    fn is_far_side(
        mouse_pos: Vec2,
        child_pos: Vec2,
        is_vertical: bool,
    ) -> bool {
        let diff = if is_vertical {
            mouse_pos.y - child_pos.y
        } else {
            mouse_pos.x - child_pos.x
        };

        diff > 0.0
    }
}
