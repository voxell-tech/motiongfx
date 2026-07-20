//! Scene hierarchy browser: an indented list of the scene's entities.
//!
//! Only *scene* entities are listed: anything with a [`Transform`]
//! that isn't part of the editor's own `bevy_ui` tree, so the panel
//! shows the composition's objects rather than the editor chrome.
//!
//! The row list is derived inside the watcher rather than by a
//! separate system writing a resource. A predicate only gets `&World`,
//! so the queries are built once at registration (where `&mut World`
//! is available) and then driven with `iter_manual`, which keeps the
//! iteration archetype-filtered instead of scanning every entity.
//!
//! The predicate computes the rows to decide whether anything changed,
//! so it hands them to the builder through a shared cache rather than
//! making it recompute them.

use std::sync::{Arc, Mutex};

use bevy::ecs::query::QueryState;
use bevy::prelude::*;
use motiongfx_editor_ui::glass::Glass;
use motiongfx_editor_ui::reactive::{BevyUi, BevyUiExt};
use motiongfx_editor_ui::theme::EditorTheme;

use crate::PANEL_PADDING;
use crate::scene::TrackViewportCamera;

/// Indent per hierarchy level.
const INDENT: f32 = 12.0;

/// Marks the scrollable panel the rows are built into.
#[derive(Component, Default, Clone)]
pub(crate) struct HierarchyPanel;

/// One row: an entity's depth and display name.
#[derive(Clone, PartialEq)]
struct Row {
    depth: usize,
    name: String,
}

/// Scene entities: transform-bearing and not editor UI.
type SceneEntity =
    (With<Transform>, Without<Node>, Without<TrackViewportCamera>);

/// The queries the predicate drives.
struct HierarchyQueries {
    scene:
        QueryState<(Entity, Option<&'static Children>), SceneEntity>,
    names: QueryState<&'static Name>,
    parents: QueryState<&'static ChildOf>,
}

impl HierarchyQueries {
    /// `try_new` rather than `new`: a builder only ever holds
    /// `&World`. Returns `None` until every component is registered.
    fn try_new(world: &World) -> Option<Self> {
        Some(Self {
            scene: QueryState::try_new(world)?,
            names: QueryState::try_new(world)?,
            parents: QueryState::try_new(world)?,
        })
    }

    fn update(&mut self, world: &World) {
        self.scene.update_archetypes(world);
        self.names.update_archetypes(world);
        self.parents.update_archetypes(world);
    }

    fn is_scene(&self, world: &World, entity: Entity) -> bool {
        self.scene.get_manual(world, entity).is_ok()
    }
}

/// The hierarchy panel, as kernel nodes.
pub(crate) fn panel(ui: &mut BevyUi) {
    let rows: Arc<Mutex<Vec<Row>>> = Arc::default();
    let seen = rows.clone();
    let mut queries: Option<HierarchyQueries> = None;

    ui.bsn(bsn! {
        HierarchyPanel
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            padding: UiRect::all(Val::Px(PANEL_PADDING)),
            overflow: Overflow::scroll_y(),
        }
        template_value(Glass::Panel)
    })
    .watch(
        move |world, _| {
            let queries = match &mut queries {
                Some(queries) => queries,
                slot => match HierarchyQueries::try_new(world) {
                    Some(queries) => slot.insert(queries),
                    None => return false,
                },
            };
            let current = collect_rows(world, queries);
            let mut seen = seen.lock().unwrap();
            let changed = *seen != current;
            *seen = current;
            changed
        },
        move |ui| {
            let rows = rows.lock().unwrap();
            build_rows(ui, &rows);
        },
    );
}

/// Roots first (a scene entity whose parent isn't itself a scene
/// entity), then depth-first so children follow their parent.
fn collect_rows(
    world: &World,
    queries: &mut HierarchyQueries,
) -> Vec<Row> {
    queries.update(world);

    let mut roots = queries
        .scene
        .iter_manual(world)
        .map(|(entity, _)| entity)
        .filter(|&entity| {
            !queries.parents.get_manual(world, entity).is_ok_and(
                |parent| queries.is_scene(world, parent.parent()),
            )
        })
        .collect::<Vec<_>>();
    roots.sort_unstable();

    let mut rows = Vec::new();
    for root in roots {
        push_subtree(world, queries, root, 0, &mut rows);
    }
    rows
}

/// Depth-first append `entity` and its scene descendants.
fn push_subtree(
    world: &World,
    queries: &HierarchyQueries,
    entity: Entity,
    depth: usize,
    out: &mut Vec<Row>,
) {
    let Ok((_, children)) = queries.scene.get_manual(world, entity)
    else {
        return;
    };
    let name = queries
        .names
        .get_manual(world, entity)
        .map(|name| name.as_str().to_string())
        .unwrap_or_else(|_| format!("Entity {}", entity.index()));
    out.push(Row { depth, name });

    let children = children
        .map(|children| children.to_vec())
        .unwrap_or_default();
    for child in children {
        push_subtree(world, queries, child, depth + 1, out);
    }
}

fn build_rows(ui: &mut BevyUi, rows: &[Row]) {
    let text_color =
        ui.world().resource::<EditorTheme>().text_primary;

    for row in rows {
        let indent = row.depth as f32 * INDENT;
        let name = row.name.clone();
        ui.bsn(bsn! {
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                padding: UiRect::left(Val::Px(indent)),
            }
        })
        .with(move |ui| {
            ui.bsn(bsn! {
                Text({name})
                TextFont { font_size: FontSize::Px(12.0) }
                TextColor({text_color})
            });
        });
    }
}
