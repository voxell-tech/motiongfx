//! Scene hierarchy browser: an indented list of the scene's subjects.
//!
//! What counts as one is an [`EntityUid`], which is the id a scene
//! refers to its subjects by - so the panel lists exactly what the
//! animation can address, and nothing the editor spawned for itself.
//!
//! Each subject watches only its own children, so adding one rebuilds
//! that branch and not the panel. Depth is the nesting: a subtree
//! indents what it holds.

mod drag;

pub(crate) use drag::Dragging;

use bevy::ecs::query::QueryState;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy_fynix::ElementMutExt;
use bevy_motiongfx::scene::id::EntityUid;
use fynix_mock::composer::Composer;
use fynix_mock::ui::{ElementHandle, ElementMut};
use fynix_mock::{elem, val};
use moxie_ui::elements::{
    ButtonElem, ButtonElemCursor, Frame, FrameCursor, GhostButton,
    Icon, Label, LabelCursor, Panel, ScrollArea, TintButton,
};
use moxie_ui::fold::{Foldable, Toggle};
use moxie_ui::reactive::{
    BevyHost, BevyUi, component_changed_on, value_changed,
};
use moxie_ui::theme::EditorTheme;

use super::PANEL_PADDING;
use crate::{SceneRoot, SelectedEntity};

/// The line marking where a drop would land a row.
const GAP: f32 = 2.0;

/// What that line answers to. Twice its own thickness, so aiming
/// between two rows does not ask for the pixels the line is drawn in -
/// the way a split's handle is wider than the seam it draws.
///
/// It holds that space whether or not a drag is happening, which is
/// also what separates one row from the next: a line that appeared
/// only when aimed at would shift every row below it as it did.
const GAP_HIT: f32 = GAP * 2.0;

/// Room below the last row for the button that floats over it.
const BUTTON_CLEARANCE: f32 = 34.0;

/// Every scene subject, as nested rows, under what acts on the list
/// as a whole.
pub(super) struct HierarchyPanel;

impl Composer<BevyHost> for HierarchyPanel {
    type Element = Panel;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Panel> {
        ui.elem(elem!(Panel, direction = FlexDirection::Column))
            .with(|ui| {
                ui.compose(Roots);
                ui.compose(AddButton);
            })
            .handle()
    }
}

/// The one thing that acts on the list rather than on a row in it.
///
/// Floated over the corner rather than given a strip of its own, so it
/// stays put however far the list is scrolled.
struct AddButton;

impl Composer<BevyHost> for AddButton {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        let accent = ui.world.resource::<EditorTheme>().accent;

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
                !TintButton,
                icon = val!(
                    Icon,
                    image = crate::icons::PLUS,
                    color = accent
                )
            ))
            .observe(
                |_: On<Activate>, mut commands: Commands| {
                    commands.queue(create);
                },
            );
        })
        .handle()
    }
}

/// The subjects themselves, scrolling under the button.
struct Roots;

impl Composer<BevyHost> for Roots {
    type Element = ScrollArea;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, ScrollArea> {
        // Roots only - a branch minds itself. The query is kept
        // because this polls every flush; the build makes its own,
        // and runs far less often.
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

/// Spawns a subject at the top level, and selects it so the inspector
/// is already pointed at what was just made.
///
/// Nothing of the animation changes: a [`Stage`](motiongfx_scene::scene::Stage)
/// seeds the fields an action drives, and a subject with no action on
/// it keeps whatever it was spawned holding.
fn create(world: &mut World) {
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

/// One list of rows, with the gaps a drop can land between them.
///
/// A gap above each row and one below the last: every place a row
/// could go gets exactly one, rather than each row bringing its own
/// pair and doubling them up in between.
fn listing(ui: &mut BevyUi, entities: Vec<Entity>) {
    let accent = ui.world.resource::<EditorTheme>().accent;

    for &entity in &entities {
        gap(ui, entity, accent, drag::At::Before);
        ui.compose(Subtree { entity });
    }

    if let Some(&last) = entities.last() {
        gap(ui, last, accent, drag::At::After);
    }
}

/// Where a drop lands a row beside another, as a hit area wider than
/// the line it commits to - the way a split's handle is wider than the
/// seam it draws.
fn gap(ui: &mut BevyUi, entity: Entity, accent: Color, at: drag::At) {
    let marker = ui.elem(elem!(
        Frame,
        width = percent(100),
        height = px(GAP_HIT),
        justify = JustifyContent::Center,
        direction = FlexDirection::Column
    ));

    drag::drops(marker, entity, at).with(move |ui| {
        ui.elem(elem!(Frame, width = percent(100), height = px(GAP)))
            .bind(
                |line| line.background(),
                drop_changed(entity, at),
                move |world, _| {
                    if world
                        .resource::<drag::Dragging>()
                        .shows(entity, at)
                    {
                        accent
                    } else {
                        Color::NONE
                    }
                },
            );
    });
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
        let text = theme.text_primary;

        ui.compose(Foldable {
            header: elem!(
                !GhostButton,
                width = percent(100),
                height = px(18),
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
            toggle: Toggle::Chevron,
            enabled: has_children(ui.world, entity),
            on_header: move |header: ElementMut<
                '_,
                '_,
                BevyHost,
                ButtonElem,
            >| {
                drag::rows(header, entity)
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
                        move |world, _| {
                            highlight(world, entity, &theme)
                        },
                    )
                    .bind(
                        |button| button.label().text(),
                        component_changed_on::<Name>(entity),
                        move |world, _| name_of(world, entity),
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
                        listing(ui, children_of(ui.world, entity));
                    },
                );
            },
        })
        .handle()
    }
}

/// What a row's own surface says: a drop landing inside it beats
/// whether it is selected, and most rows are neither.
fn highlight(
    world: &World,
    entity: Entity,
    theme: &EditorTheme,
) -> Color {
    if world
        .resource::<drag::Dragging>()
        .shows(entity, drag::At::Into)
    {
        theme.accent.with_alpha(0.35)
    } else if world.resource::<SelectedEntity>().0 == Some(entity) {
        theme.accent.with_alpha(0.18)
    } else {
        Color::NONE
    }
}

/// Fires when either half of what [`highlight`] reads moves.
fn highlight_changed(
    entity: Entity,
) -> impl FnMut(&World, Entity) -> bool {
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
) -> impl FnMut(&World, Entity) -> bool {
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
/// them - a row only needs to know that there is something to fold.
fn has_children(world: &World, entity: Entity) -> bool {
    world.get::<Children>(entity).is_some_and(|children| {
        children.iter().any(|child| is_subject(world, child))
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
