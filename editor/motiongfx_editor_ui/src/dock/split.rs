//! Resizable split panels: a [`PanelGroup`] arranges its [`Panel`]
//! children along its flex axis according to each panel's `ratio`,
//! with a draggable [`PanelHandle`] between adjacent panels. Dragging
//! a handle also mirrors the new fraction into the
//! [`DockTree`](super::tree::DockTree) split it's bound to (via
//! [`super::reconcile::NodeBinding`]), so the reconciler and the live
//! layout stay in sync.

use bevy::ecs::spawn::SpawnableList;
use bevy::feathers::cursor::{
    CursorIconPlugin, EntityCursor, OverrideCursor,
};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::ui::{UiGlobalTransform, UiScale};
use bevy::window::SystemCursorIcon;

pub struct SplitPanelPlugin;

impl Plugin for SplitPanelPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<CursorIconPlugin>() {
            app.add_plugins(CursorIconPlugin);
        }

        app.add_observer(on_panel_added)
            .add_observer(on_handle_added)
            .add_observer(on_handle_drag_start)
            .add_observer(on_handle_drag_end)
            .add_observer(on_handle_hover)
            .add_observer(on_handle_unhover)
            .add_observer(handle_panel_drag)
            .add_systems(Update, recalculate_changed_panels);
    }
}

const HANDLE_SIZE: f32 = 3.0;
const HANDLE_HOVER_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.12);

#[derive(Component)]
pub struct PanelGroup {
    pub min_ratio: f32,
}

#[derive(Component)]
pub struct Panel {
    pub ratio: f32,
}

#[derive(Component)]
pub struct PanelHandle;

/// Present on a handle while it's being dragged, so the highlight
/// stays visible for the whole drag even if the cursor leaves it.
#[derive(Component)]
struct HandleDragging;

pub fn panel_group<
    C: SpawnableList<ChildOf> + Send + Sync + 'static,
>(
    min_ratio: f32,
    panels: C,
) -> impl Bundle {
    (PanelGroup { min_ratio }, Children::spawn(panels))
}

pub fn panel(ratio: f32) -> impl Bundle {
    Panel { ratio }
}

pub fn panel_handle() -> impl Bundle {
    (
        PanelHandle,
        Node {
            min_width: Val::Px(HANDLE_SIZE),
            min_height: Val::Px(HANDLE_SIZE),
            ..default()
        },
        BackgroundColor(Color::NONE),
    )
}

fn on_panel_added(
    trigger: On<Add, Panel>,
    child_of: Query<&ChildOf>,
    mut queries: ParamSet<(
        Query<(&Node, &Children), With<PanelGroup>>,
        Query<(&mut Node, &Panel)>,
    )>,
) {
    let entity = trigger.event_target();
    let Ok(&ChildOf(parent)) = child_of.get(entity) else {
        return;
    };
    recalculate_group(parent, &mut queries);
}

fn recalculate_changed_panels(
    changed: Query<&ChildOf, Changed<Panel>>,
    mut queries: ParamSet<(
        Query<(&Node, &Children), With<PanelGroup>>,
        Query<(&mut Node, &Panel)>,
    )>,
) {
    let mut seen = HashSet::new();
    for parent_ref in &changed {
        let parent = parent_ref.parent();
        if seen.insert(parent) {
            recalculate_group(parent, &mut queries);
        }
    }
}

fn recalculate_group(
    group_entity: Entity,
    queries: &mut ParamSet<(
        Query<(&Node, &Children), With<PanelGroup>>,
        Query<(&mut Node, &Panel)>,
    )>,
) {
    let groups = queries.p0();
    let Ok((group_node, children)) = groups.get(group_entity) else {
        return;
    };
    let flex_direction = group_node.flex_direction;
    let child_entities: Vec<Entity> = children.iter().collect();

    // Sum only visible panels. Hidden (Display::None) panels are out
    // of layout, so giving them a percentage steals space from
    // siblings.
    let panels_ro = queries.p1();
    let total: f32 = panels_ro
        .iter_many(&child_entities)
        .filter(|(node, _)| node.display != Display::None)
        .map(|(_, panel)| panel.ratio)
        .sum();

    if total <= 0.0 {
        return;
    }

    let mut panels = queries.p1();
    let mut iterator = panels.iter_many_mut(&child_entities);
    while let Some((mut node, panel)) = iterator.fetch_next() {
        if node.display == Display::None {
            continue;
        }
        let pct = (panel.ratio / total) * 100.;
        match flex_direction {
            FlexDirection::Row | FlexDirection::RowReverse => {
                node.width = Val::Percent(pct);
                node.min_width = Val::Px(0.0);
            }
            FlexDirection::Column | FlexDirection::ColumnReverse => {
                node.height = Val::Percent(pct);
                node.min_height = Val::Px(0.0);
            }
        }
    }
}

fn on_handle_added(
    trigger: On<Add, PanelHandle>,
    handles: Query<&ChildOf, With<PanelHandle>>,
    nodes: Query<&Node>,
    mut commands: Commands,
) {
    let Ok(&ChildOf(parent)) = handles.get(trigger.entity) else {
        return;
    };
    let Ok(node) = nodes.get(parent) else {
        return;
    };
    let cursor_icon = get_drag_icon(node.flex_direction);
    commands
        .entity(trigger.entity)
        .insert(EntityCursor::System(cursor_icon));
}

fn on_handle_drag_start(
    trigger: On<Pointer<DragStart>>,
    handles: Query<&ChildOf, With<PanelHandle>>,
    nodes: Query<&Node>,
    mut override_cursor: ResMut<OverrideCursor>,
    mut commands: Commands,
) {
    let handle = trigger.event_target();
    let Ok(&ChildOf(parent)) = handles.get(handle) else {
        return;
    };
    // Keep the highlight shown for the whole drag.
    commands.entity(handle).insert((
        HandleDragging,
        BackgroundColor(HANDLE_HOVER_COLOR),
    ));
    let Ok(node) = nodes.get(parent) else {
        return;
    };
    let cursor_icon = get_drag_icon(node.flex_direction);
    if override_cursor.is_none() {
        override_cursor.0 = Some(EntityCursor::System(cursor_icon));
    }
}

fn on_handle_drag_end(
    trigger: On<Pointer<DragEnd>>,
    handles: Query<&ChildOf, With<PanelHandle>>,
    nodes: Query<&Node>,
    mut override_cursor: ResMut<OverrideCursor>,
    mut commands: Commands,
) {
    let handle = trigger.event_target();
    let Ok(&ChildOf(parent)) = handles.get(handle) else {
        return;
    };
    // End of drag: drop the marker and clear the highlight.
    commands
        .entity(handle)
        .remove::<HandleDragging>()
        .insert(BackgroundColor(Color::NONE));
    let Ok(node) = nodes.get(parent) else {
        return;
    };
    let cursor_icon = get_drag_icon(node.flex_direction);
    if override_cursor.0 == Some(EntityCursor::System(cursor_icon)) {
        override_cursor.0 = None;
    }
}

fn on_handle_hover(
    trigger: On<Pointer<Over>>,
    handles: Query<(), With<PanelHandle>>,
    mut commands: Commands,
) {
    if handles.get(trigger.event_target()).is_ok() {
        commands
            .entity(trigger.event_target())
            .insert(BackgroundColor(HANDLE_HOVER_COLOR));
    }
}

fn on_handle_unhover(
    trigger: On<Pointer<Out>>,
    // Skip while dragging so leaving the handle keeps the highlight.
    handles: Query<(), (With<PanelHandle>, Without<HandleDragging>)>,
    mut commands: Commands,
) {
    if handles.get(trigger.event_target()).is_ok() {
        commands
            .entity(trigger.event_target())
            .insert(BackgroundColor(Color::NONE));
    }
}

fn handle_panel_drag(
    mut drag: On<Pointer<Drag>>,
    handles: Query<&ChildOf, With<PanelHandle>>,
    groups: Query<(&PanelGroup, &Node, &Children)>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    bindings: Query<&super::reconcile::NodeBinding>,
    mut tree: ResMut<super::tree::DockTree>,
    mut panels: Query<&mut Panel>,
    ui_scale: Res<UiScale>,
) {
    let handle_entity = drag.event_target();
    let Ok(&ChildOf(parent)) = handles.get(handle_entity) else {
        return;
    };
    let Ok((group, node, children)) = groups.get(parent) else {
        return;
    };

    let Some(handle_index) =
        children.iter().position(|e| e == handle_entity)
    else {
        return;
    };
    if handle_index == 0 || handle_index + 1 >= children.len() {
        return;
    }
    let before_entity = children[handle_index - 1];
    let after_entity = children[handle_index + 1];

    // Track the cursor's absolute position within the two panels'
    // span, not accumulated delta — so dragging off-screen or past a
    // limit just clamps and holds rather than banking up movement.
    let (Ok((bc, bt)), Ok((ac, at))) =
        (nodes.get(before_entity), nodes.get(after_entity))
    else {
        return;
    };
    let before_rect = super::drag::logical_rect(bc, bt);
    let after_rect = super::drag::logical_rect(ac, at);
    let cursor = drag.pointer_location.position / ui_scale.0;

    let vertical = matches!(
        node.flex_direction,
        FlexDirection::Column | FlexDirection::ColumnReverse
    );
    let reverse = matches!(
        node.flex_direction,
        FlexDirection::RowReverse | FlexDirection::ColumnReverse
    );
    let (span_min, span_max, cur) = if vertical {
        (
            before_rect.min.y.min(after_rect.min.y),
            before_rect.max.y.max(after_rect.max.y),
            cursor.y,
        )
    } else {
        (
            before_rect.min.x.min(after_rect.min.x),
            before_rect.max.x.max(after_rect.max.x),
            cursor.x,
        )
    };
    let span = span_max - span_min;
    if span <= 0.0 {
        return;
    }

    let total: f32 = panels
        .get_many([before_entity, after_entity])
        .map(|[b, a]| b.ratio + a.ratio)
        .unwrap_or(0.0);
    if total <= 0.0 {
        return;
    }

    // `before`'s fraction of the span (flipped for reverse layouts),
    // clamped so neither panel drops below `min_ratio`.
    let mut p = (cur - span_min) / span;
    if reverse {
        p = 1.0 - p;
    }
    let min_p = (group.min_ratio / total).clamp(0.0, 0.5);
    p = p.clamp(min_p, 1.0 - min_p);

    let Ok([mut before, mut after]) =
        panels.get_many_mut([before_entity, after_entity])
    else {
        return;
    };
    before.ratio = total * p;
    after.ratio = total * (1.0 - p);

    // Mirror the fraction into a bound tree split so the reconciler
    // stays in sync.
    if let Ok(binding) = bindings.get(parent) {
        tree.set_fraction(binding.0, p);
    }

    drag.propagate(false);
}

fn get_drag_icon(direction: FlexDirection) -> SystemCursorIcon {
    match direction {
        FlexDirection::Row | FlexDirection::RowReverse => {
            SystemCursorIcon::ColResize
        }
        FlexDirection::Column | FlexDirection::ColumnReverse => {
            SystemCursorIcon::RowResize
        }
    }
}
