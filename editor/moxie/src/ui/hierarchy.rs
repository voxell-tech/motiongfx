//! Scene hierarchy browser: an indented list of the scene's subjects.
//!
//! What counts as one is an [`EntityUid`], which is the id a scene
//! refers to its subjects by - so the panel lists exactly what the
//! animation can address, and nothing the editor spawned for itself.
//!
//! Each subject watches only its own children, so adding one rebuilds
//! that branch and not the panel. Depth is the nesting: a subtree
//! indents what it holds.

use bevy::ecs::query::QueryState;
use bevy::prelude::*;
use bevy_motiongfx::scene::id::EntityUid;
use fynix_mock::composer::Composer;
use fynix_mock::elem;
use fynix_mock::ui::ElementHandle;
use moxie_ui::elements::{Frame, Label, LabelCursor, Panel};
use moxie_ui::reactive::{
    BevyHost, BevyUi, component_changed_on, value_changed,
};
use moxie_ui::theme::EditorTheme;

use super::PANEL_PADDING;

/// How far a subject sets its children in from itself.
const INDENT: f32 = 12.0;

/// Between one row and the next, at any depth.
const ROW_GAP: f32 = 2.0;

/// Every scene subject, as nested rows.
pub(super) struct HierarchyPanel;

impl Composer<BevyHost> for HierarchyPanel {
    type Element = Panel;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Panel> {
        // Roots only - a branch minds itself. The query is kept
        // because this polls every flush; the build makes its own,
        // and runs far less often.
        let mut query = None;
        let mut seen: Option<Vec<Entity>> = None;

        ui.elem(elem!(
            Panel,
            direction = FlexDirection::Column,
            row_gap = px(ROW_GAP),
            padding = UiRect::all(px(PANEL_PADDING)),
            scrolls = true
        ))
        .watch(
            move |world, _| {
                let query = match &mut query {
                    Some(query) => query,
                    slot => match QueryState::try_new(world) {
                        Some(query) => slot.insert(query),
                        None => return false,
                    },
                };
                query.update_archetypes(world);

                let current = roots(world, query);
                let changed = seen.as_ref() != Some(&current);
                seen = Some(current);
                changed
            },
            build_roots,
        )
        .handle()
    }
}

fn build_roots(ui: &mut BevyUi) {
    let roots = {
        let Some(mut query) = QueryState::try_new(ui.world) else {
            return;
        };
        roots(ui.world, &mut query)
    };

    for entity in roots {
        ui.compose(Subtree { entity });
    }
}

/// One subject, and everything under it.
struct Subtree {
    entity: Entity,
}

impl Composer<BevyHost> for Subtree {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        let entity = self.entity;

        ui.elem(elem!(
            Frame,
            width = percent(100),
            direction = FlexDirection::Column,
            row_gap = px(ROW_GAP)
        ))
        .with(move |ui| {
            ui.compose(Subject { entity });

            // Indented, watching only whose children they are.
            ui.elem(elem!(
                Frame,
                width = percent(100),
                direction = FlexDirection::Column,
                row_gap = px(ROW_GAP),
                padding = UiRect::new(
                    px(INDENT),
                    Val::ZERO,
                    Val::ZERO,
                    Val::ZERO
                )
            ))
            .watch(
                component_changed_on::<Children>(entity),
                move |ui| {
                    for child in children_of(ui.world, entity) {
                        ui.compose(Subtree { entity: child });
                    }
                },
            );
        })
        .handle()
    }
}

/// One subject's own row.
struct Subject {
    entity: Entity,
}

impl Composer<BevyHost> for Subject {
    type Element = Label;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Label> {
        let entity = self.entity;
        let color = ui.world.resource::<EditorTheme>().text_primary;
        let name = name_of(ui.world, entity);

        // Bound, not rebuilt: a rename changes no shape.
        ui.elem(elem!(Label, text = name, color = Some(color)))
            .bind(
                |label| label.text(),
                value_changed(move |world: &World, _| {
                    name_of(world, entity)
                }),
                move |world, _| name_of(world, entity),
            )
            .handle()
    }
}

/// Subjects with no subject above them. Sorted, so the panel does not
/// reshuffle when an archetype does.
fn roots(
    world: &World,
    query: &mut QueryState<Entity, With<EntityUid>>,
) -> Vec<Entity> {
    let mut roots: Vec<Entity> = query
        .iter_manual(world)
        .filter(|&entity| {
            !world.get::<ChildOf>(entity).is_some_and(|parent| {
                is_subject(world, parent.parent())
            })
        })
        .collect();

    roots.sort_unstable();
    roots
}

/// The subjects directly under `entity`; anything else it parents is
/// not the scene's to show.
fn children_of(world: &World, entity: Entity) -> Vec<Entity> {
    world
        .get::<Children>(entity)
        .map(|children| {
            children
                .iter()
                .filter(|&child| is_subject(world, child))
                .collect()
        })
        .unwrap_or_default()
}

fn is_subject(world: &World, entity: Entity) -> bool {
    world.get::<EntityUid>(entity).is_some()
}

/// Its [`Name`], or the head of its id - a whole uuid is unreadable,
/// and the first characters tell two unnamed subjects apart.
fn name_of(world: &World, entity: Entity) -> String {
    /// How much of an id a row shows.
    const HEAD: usize = 8;

    if let Some(name) = world.get::<Name>(entity) {
        return name.as_str().to_string();
    }

    match world.get::<EntityUid>(entity) {
        Some(uid) => uid.to_string().chars().take(HEAD).collect(),
        None => "?".to_string(),
    }
}
