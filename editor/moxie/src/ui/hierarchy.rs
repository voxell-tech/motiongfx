//! Scene hierarchy browser: an indented list of the scene's subjects.
//!
//! What counts as one is an [`EntityUid`], which is the id a scene
//! refers to its subjects by - so the panel lists exactly what the
//! animation can address, and nothing the editor spawned for itself.

use std::sync::{Arc, Mutex};

use bevy::ecs::query::QueryState;
use bevy::prelude::*;
use bevy_motiongfx::scene::id::EntityUid;
use fynix_mock::composer::Composer;
use fynix_mock::elem;
use fynix_mock::ui::ElementHandle;
use moxie_ui::elements::{Frame, Label, Panel};
use moxie_ui::reactive::{BevyHost, BevyUi};
use moxie_ui::theme::EditorTheme;

use super::PANEL_PADDING;

/// Indent per hierarchy level.
const INDENT: f32 = 12.0;

/// One row: an entity's depth and display name.
#[derive(Clone, PartialEq)]
struct Row {
    depth: usize,
    name: String,
}

/// A scene subject is whatever carries the id one is named by.
type SceneEntity = With<EntityUid>;

/// The queries the predicate drives.
struct HierarchyQueries {
    scene:
        QueryState<(Entity, Option<&'static Children>), SceneEntity>,
    names: QueryState<(Option<&'static Name>, &'static EntityUid)>,
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

    /// What the row calls it: its [`Name`] if it was given one, and
    /// otherwise the head of its id - a whole uuid is unreadable at
    /// a glance, and its first characters are enough to tell two
    /// unnamed subjects apart.
    fn name_of(&self, world: &World, entity: Entity) -> String {
        /// As many characters of an id as a row shows.
        const HEAD: usize = 8;

        match self.names.get_manual(world, entity) {
            Ok((Some(name), _)) => name.as_str().to_string(),
            Ok((None, uid)) => {
                uid.to_string().chars().take(HEAD).collect()
            }
            Err(_) => "?".to_string(),
        }
    }

    fn is_scene(&self, world: &World, entity: Entity) -> bool {
        self.scene.get_manual(world, entity).is_ok()
    }
}

/// The hierarchy panel, as kernel nodes.
pub(super) struct HierarchyPanel;

impl Composer<BevyHost> for HierarchyPanel {
    type Element = Panel;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Panel> {
        // The rows the predicate found, handed to the build that
        // follows it: collecting them twice would walk the whole
        // scene twice per change.
        let rows: Arc<Mutex<Vec<Row>>> = Arc::default();
        let seen = rows.clone();
        let mut queries: Option<HierarchyQueries> = None;

        ui.elem(elem!(
            Panel,
            direction = FlexDirection::Column,
            row_gap = px(2),
            padding = UiRect::all(px(PANEL_PADDING)),
            scrolls = true
        ))
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
        )
        .handle()
    }
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
    let name = queries.name_of(world, entity);
    out.push(Row { depth, name });

    let children = children
        .map(|children| children.to_vec())
        .unwrap_or_default();
    for child in children {
        push_subtree(world, queries, child, depth + 1, out);
    }
}

fn build_rows(ui: &mut BevyUi, rows: &[Row]) {
    let text_color = ui.world.resource::<EditorTheme>().text_primary;

    for row in rows {
        let indent = row.depth as f32 * INDENT;
        let name = row.name.clone();
        ui.elem(elem!(
            Frame,
            width = percent(100),
            align = AlignItems::Center,
            padding = UiRect::left(px(indent))
        ))
        .with(move |ui| {
            ui.elem(elem!(
                Label,
                text = name,
                color = Some(text_color)
            ));
        });
    }
}
