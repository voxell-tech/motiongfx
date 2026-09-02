//! Scene hierarchy browser: an indented list of the scene's subjects.
//!
//! What counts as one is an [`EntityUid`], the id a scene refers to
//! its subjects by. The panel lists exactly what the animation can
//! address, nothing the editor spawned for itself.
//!
//! Each subject watches only its own children, so adding one rebuilds
//! that branch and not the panel. Depth is the nesting: a subtree
//! indents what it holds.

mod drag;

pub(crate) use drag::Dragging;

use bevy::ecs::query::QueryState;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy_fynix::WorldEntityMut;
use bevy_motiongfx::scene::id::EntityUid;
use fynix::WorldNodeRef;
use fynix::composer::Composer;
use fynix::ui::{ElementHandle, ElementMut};
use fynix::{elem, val};
use moxie_ui::elements::{
    ButtonElem, ButtonElemCursor, Frame, FrameCursor, GhostButton,
    Icon, Label, LabelCursor, Panel, ScrollArea, TintButton,
};
use moxie_ui::fold::{Foldable, FoldsOn};
use moxie_ui::reactive::{
    BevyUi, FynixHost, component_changed_on, value_changed,
};

use super::PANEL_PADDING;
use crate::{SceneRoot, SelectedEntity};

/// The line marking where a drop would land a row, beside it. Drawn
/// over the seam between rows, taking no layout space of its own, so
/// the rows do not shift as it appears.
const DROP_LINE: f32 = 2.0;

/// A subject's own row, without anything folded out under it. The
/// drop-after line hangs at its foot, which is the next sibling's
/// place when the row is shut and its first child's when it is open.
const ROW_HEIGHT: f32 = 18.0;

/// Room below the last row for the button that floats over it.
const BUTTON_CLEARANCE: f32 = 34.0;

/// Every scene subject, as nested rows, under what acts on the list
/// as a whole.
pub(super) struct HierarchyPanel;

impl Composer<FynixHost> for HierarchyPanel {
    type Element = Panel;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, Panel> {
        ui.elem(elem!(Panel))
            .with(|ui| {
                ui.compose(Roots);
                ui.compose(AddButton);
            })
            .handle()
    }
}

/// The one thing that acts on the list, not on a row in it.
///
/// Floated over the corner, not given a strip of its own, so it
/// stays put however far the list is scrolled.
struct AddButton;

impl Composer<FynixHost> for AddButton {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, Frame> {
        ui.elem(elem!(
            Frame,
            position = PositionType::Absolute,
            inset = UiRect::new(
                auto(),
                px(PANEL_PADDING),
                auto(),
                px(PANEL_PADDING)
            )
        ))
        .with(move |ui| {
            ui.elem(elem!(
                !TintButton::default(),
                icon = val!(Icon, image = crate::icons::PLUS)
            ))
            .observe(
                |_: On<Activate>, mut commands: Commands| {
                    commands.queue(spawn_new_entity);
                },
            );
        })
        .handle()
    }
}

/// The subjects themselves, scrolling under the button.
struct Roots;

impl Composer<FynixHost> for Roots {
    type Element = ScrollArea;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, ScrollArea> {
        // Roots only: a branch minds itself. The query is kept
        // because this polls every flush, and the build (which
        // makes its own) runs far less often.
        let mut query = None;
        let mut seen: Option<Vec<Entity>> = None;

        ui.elem(elem!(
            ScrollArea,
            width = percent(100),
            flex_grow = 1.0f32,
            padding = UiRect::new(
                px(PANEL_PADDING),
                px(PANEL_PADDING),
                px(PANEL_PADDING),
                px(BUTTON_CLEARANCE)
            ),
            scroll_x = false
        ))
        .watch(
            move |WorldNodeRef { world, .. }| {
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

/// Spawns a subject at the top level, and selects it so the inspector
/// is already pointed at what was just made.
///
/// Nothing of the animation changes: a [`Stage`](motiongfx_scene::scene::Stage)
/// seeds the fields an action drives, and a subject with no action on
/// it keeps whatever it was spawned holding.
fn spawn_new_entity(world: &mut World) {
    let Ok(root) = world
        .query_filtered::<Entity, With<SceneRoot>>()
        .single(world)
    else {
        error!("Scene root does not exist!");
        return;
    };

    let entity = world
        .spawn((
            EntityUid::new(),
            Name::new("Entity"),
            Transform::default(),
            Visibility::default(),
            ChildOf(root),
        ))
        .id();

    world.insert_resource(SelectedEntity(Some(entity)));
}

fn build_roots(ui: &mut BevyUi) {
    let roots = {
        let Some(mut query) = QueryState::try_new(ui.world) else {
            return;
        };
        roots(ui.world, &mut query)
    };

    listing(ui, roots);
}

/// One list of subtrees, each carrying the lines a drop can land
/// before or after it.
fn listing(ui: &mut BevyUi, entities: Vec<Entity>) {
    for entity in entities {
        ui.compose(Subtree { entity });
    }
}

/// The line a drop would land on to place a row beside `entity`, at
/// `at`. Absolutely positioned against the subject's own row, so it
/// costs no height, the rows do not shift as it shows, and it stays at
/// that row's edge whether or not children are folded out below it.
fn drop_line(ui: &mut BevyUi, entity: Entity, at: drag::At) {
    let accent = ui.theme.accent;
    let top = match at {
        drag::At::Before => px(-DROP_LINE / 2.0),
        _ => px(ROW_HEIGHT - DROP_LINE / 2.0),
    };

    ui.elem(elem!(
        Frame,
        position = PositionType::Absolute,
        inset = UiRect {
            left: px(0),
            right: px(0),
            top,
            bottom: auto()
        },
        height = px(DROP_LINE)
    ))
    .bind(
        |line| line.background(),
        drop_changed(entity, at),
        move |WorldNodeRef { world, .. }| {
            if world.resource::<drag::Dragging>().shows(entity, at) {
                accent
            } else {
                Color::NONE
            }
        },
    );
}

/// One subject, and everything under it.
struct Subtree {
    entity: Entity,
}

impl Composer<FynixHost> for Subtree {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, Frame> {
        let entity = self.entity;

        // A relative wrapper the drop lines anchor to, so they mark
        // this subject's own row without being spaced apart from it.
        ui.elem(elem!(
            Frame,
            width = percent(100),
            direction = FlexDirection::Column
        ))
        .with(move |ui| {
            drop_line(ui, entity, drag::At::Before);

            let name = name_of(ui.world, entity);
            let text = ui.theme.text_primary;
            let accent = ui.theme.accent;

            ui.compose(Foldable {
                header: elem!(
                    !GhostButton,
                    width = percent(100),
                    height = px(ROW_HEIGHT),
                    justify = JustifyContent::FlexStart,
                    padding = UiRect::axes(px(4), Val::ZERO),
                    radius = px(3),
                    label = val!(
                        Label,
                        text = name,
                        wrap = false,
                        color = Some(text)
                    )
                ),
                // The row is the subject's, to select; only the
                // chevron beside it folds.
                folds_on: FoldsOn::Chevron,
                enabled: has_children(ui.world, entity),
                on_header: move |mut header: ElementMut<
                    '_,
                    '_,
                    FynixHost,
                    ButtonElem,
                >| {
                    drag::rows(&mut header, entity)
                        .observe(
                            move |_: On<Activate>,
                                  mut selected: ResMut<
                                SelectedEntity,
                            >| {
                                selected.0 = Some(entity);
                            },
                        )
                        // One bind, not two: a second on the same
                        // field would fight this one every flush.
                        .bind(
                            |button| button.fill(),
                            highlight_changed(entity),
                            move |WorldNodeRef { world, .. }| {
                                highlight(world, entity, accent)
                            },
                        )
                        .bind(
                            |button| button.label().text(),
                            component_changed_on::<Name>(entity),
                            move |WorldNodeRef { world, .. }| {
                                name_of(world, entity)
                            },
                        );
                },
                body: move |ui: &mut BevyUi| {
                    ui.elem(elem!(
                        Frame,
                        width = percent(100),
                        direction = FlexDirection::Column
                    ))
                    .watch(
                        component_changed_on::<Children>(entity),
                        move |ui| {
                            listing(
                                ui,
                                children_of(ui.world, entity),
                            );
                        },
                    );
                },
                // Read off the subject's own entity, not this row's node.
                // The row rebuilds fresh on a reorder or a sibling
                // added, but the entity, and `Collapsed` on it, does not.
                // Nothing to clean up when a subject is deleted either.
                // `Collapsed` goes with it.
                open: ui.world.get::<Collapsed>(entity).is_none(),
                on_toggle: move |world: &mut World, open: bool| {
                    let Ok(mut entity) = world.get_entity_mut(entity)
                    else {
                        return;
                    };
                    if open {
                        entity.remove::<Collapsed>();
                    } else {
                        entity.insert(Collapsed);
                    }
                },
            });

            drop_line(ui, entity, drag::At::After);
        })
        .handle()
    }
}

/// On a subject's own entity while its hierarchy row is collapsed.
/// Nothing removes this when the row's own node is despawned and
/// rebuilt, since it was never on that node. It goes when the
/// subject itself does.
#[derive(Component)]
struct Collapsed;

/// What a row's own surface says: a drop landing inside it beats
/// whether it is selected, and most rows are neither.
fn highlight(world: &World, entity: Entity, accent: Color) -> Color {
    if world
        .resource::<drag::Dragging>()
        .shows(entity, drag::At::Into)
    {
        accent.with_alpha(0.35)
    } else if world.resource::<SelectedEntity>().0 == Some(entity) {
        accent.with_alpha(0.18)
    } else {
        Color::NONE
    }
}

/// Fires when either half of what [`highlight`] reads moves.
fn highlight_changed(
    entity: Entity,
) -> impl for<'w> FnMut(WorldNodeRef<'w, FynixHost>) -> bool {
    value_changed(move |world, _| {
        (
            world.resource::<SelectedEntity>().0 == Some(entity),
            world
                .resource::<drag::Dragging>()
                .shows(entity, drag::At::Into),
        )
    })
}

/// Fires when whether a drop would land at `at` beside `entity` moves.
fn drop_changed(
    entity: Entity,
    at: drag::At,
) -> impl for<'w> FnMut(WorldNodeRef<'w, FynixHost>) -> bool {
    value_changed(move |world, _| {
        world.resource::<drag::Dragging>().shows(entity, at)
    })
}

/// The top-level subjects, in the order [`SceneRoot`] holds them.
///
/// The root itself is never a row: it exists to give the top level an
/// order, not to be seen.
fn roots(
    world: &World,
    query: &mut QueryState<Entity, With<SceneRoot>>,
) -> Vec<Entity> {
    query
        .iter_manual(world)
        .next()
        .map(|root| children_of(world, root))
        .unwrap_or_default()
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
/// them. A row only needs to know there is something to fold.
fn has_children(world: &World, entity: Entity) -> bool {
    world.get::<Children>(entity).is_some_and(|children| {
        children.iter().any(|child| is_subject(world, child))
    })
}

fn is_subject(world: &World, entity: Entity) -> bool {
    world.get::<EntityUid>(entity).is_some()
}

/// Its [`Name`], or the head of its id. A whole uuid is unreadable;
/// the first characters tell two unnamed subjects apart.
fn name_of(world: &World, entity: Entity) -> String {
    if let Some(name) = world.get::<Name>(entity) {
        return name.as_str().to_string();
    }

    match world.get::<EntityUid>(entity) {
        Some(uid) => uid_head(*uid),
        None => "?".to_string(),
    }
}

/// How much of an id a row shows. A whole uuid is unreadable; the
/// first characters are enough to tell two subjects apart.
pub(crate) fn uid_head(uid: EntityUid) -> String {
    const HEAD: usize = 8;
    uid.to_string().chars().take(HEAD).collect()
}
