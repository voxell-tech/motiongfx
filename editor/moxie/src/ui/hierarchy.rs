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
use bevy::ui_widgets::Activate;
use bevy_fynix::ElementMutExt;
use bevy_motiongfx::scene::id::EntityUid;
use fynix_mock::composer::Composer;
use fynix_mock::ui::{ElementHandle, ElementMut};
use fynix_mock::{elem, val};
use moxie_ui::elements::{
    ButtonElem, ButtonElemCursor, Frame, Label, Panel, TintButton,
};
use moxie_ui::fold::{Foldable, Toggle};
use moxie_ui::reactive::{
    BevyHost, BevyUi, component_changed_on, value_changed,
};
use moxie_ui::theme::EditorTheme;

use super::PANEL_PADDING;
use crate::SelectedEntity;

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
        let theme = ui.world.resource::<EditorTheme>().clone();
        let name = name_of(ui.world, entity);

        ui.compose(Foldable {
            header: elem!(
                !TintButton,
                width = percent(100),
                height = px(18),
                justify = JustifyContent::FlexStart,
                padding = UiRect::axes(px(4), Val::ZERO),
                radius = px(3),
                label = val!(
                    Label,
                    text = name,
                    wrap = false,
                    color = Some(theme.text_primary)
                )
            ),
            // The row is the subject's, to select; only the chevron
            // beside it folds.
            toggle: Toggle::Chevron,
            enabled: has_children(ui.world, entity),
            on_header: move |header: ElementMut<
                '_,
                '_,
                BevyHost,
                ButtonElem,
            >| {
                header
                    .observe(
                        move |_: On<Activate>,
                              mut selected: ResMut<
                            SelectedEntity,
                        >| {
                            selected.0 = Some(entity);
                        },
                    )
                    .bind(
                        |button| button.fill(),
                        selection_changed(entity),
                        move |world, _| {
                            if world.resource::<SelectedEntity>().0
                                == Some(entity)
                            {
                                theme.accent.with_alpha(0.18)
                            } else {
                                Color::NONE
                            }
                        },
                    );
            },
            body: move |ui: &mut BevyUi| {
                ui.elem(elem!(
                    Frame,
                    width = percent(100),
                    direction = FlexDirection::Column,
                    row_gap = px(ROW_GAP)
                ))
                .watch(
                    component_changed_on::<Children>(entity),
                    move |ui| {
                        for child in children_of(ui.world, entity) {
                            ui.compose(Subtree { entity: child });
                        }
                    },
                );
            },
        })
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

/// Whether `entity` has any subject under it, without collecting
/// them - a row only needs to know that there is something to fold.
fn has_children(world: &World, entity: Entity) -> bool {
    world.get::<Children>(entity).is_some_and(|children| {
        children.iter().any(|child| is_subject(world, child))
    })
}

/// Fires when `entity` gains or loses the hierarchy's selection.
fn selection_changed(
    entity: Entity,
) -> impl FnMut(&World, Entity) -> bool {
    value_changed(move |world, _| {
        world.resource::<SelectedEntity>().0 == Some(entity)
    })
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
