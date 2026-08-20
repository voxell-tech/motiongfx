//! Moving a subject by dragging its row.
//!
//! Reordering and reparenting are the same edit: every subject sits in
//! some parent's [`Children`], so both are a matter of which list a row
//! lands in and where. A row itself means inside it; the gaps either
//! side of one mean beside it, and are their own drop targets so that
//! what commits to a hair-thin line is not itself hair-thin.

use bevy::picking::events::{
    Drag, DragDrop, DragEnd, DragLeave, DragOver, DragStart, Pointer,
};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::UiScale;
use bevy_fynix::{BevyFynix, EntityExt};
use fynix_mock::element::Element;
use fynix_mock::ui::ElementMut;
use moxie_ui::elements::ButtonElem;
use moxie_ui::reactive::BevyHost;
use moxie_ui::theme::EditorTheme;

/// Where the ghost sits relative to the cursor, so the cursor lands
/// just inside it rather than on its corner.
const GHOST_OFFSET: Vec2 = Vec2::new(-8.0, -9.0);

/// The subject being dragged, where a drop would land it, and what is
/// following the cursor meanwhile. Empty whenever nothing is being
/// dragged.
#[derive(Resource, Default)]
pub(crate) struct Dragging {
    /// The subject, not the node it was picked up by: a pointer event
    /// names whichever node it hit, which may be a row's label rather
    /// than the row, and neither is the thing being moved.
    subject: Option<Entity>,
    target: Option<(Entity, At)>,
    ghost: Option<Entity>,
}

impl Dragging {
    /// Whether a drop would land at `at` relative to `row`.
    pub(crate) fn shows(&self, row: Entity, at: At) -> bool {
        self.target == Some((row, at))
    }
}

/// Where a drop would put what is being dragged, relative to the row
/// it is aimed at.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum At {
    Before,
    Into,
    After,
}

/// Makes `header` a row that can be picked up, and dropped into.
pub(super) fn rows<'r, 'u, 'a>(
    header: &'r mut ElementMut<'u, 'a, BevyHost, ButtonElem>,
    subject: Entity,
) -> &'r mut ElementMut<'u, 'a, BevyHost, ButtonElem> {
    drops(header, subject, At::Into);

    header
        .observe(
            move |start: On<Pointer<DragStart>>,
                  names: Query<&Name>,
                  kernel: Res<BevyFynix<EditorTheme>>,
                  scale: Res<UiScale>,
                  mut dragging: ResMut<Dragging>,
                  mut commands: Commands| {
                if start.button != PointerButton::Primary {
                    return;
                }

                let name = names
                    .get(subject)
                    .map(|name| name.as_str().to_string())
                    .unwrap_or_default();
                let at = start.pointer_location.position / scale.0;

                dragging.subject = Some(subject);
                dragging.ghost = Some(
                    commands
                        .spawn(ghost(at, name, kernel.theme()))
                        .id(),
                );
            },
        )
        .observe(
            move |drag: On<Pointer<Drag>>,
                  scale: Res<UiScale>,
                  dragging: Res<Dragging>,
                  mut nodes: Query<&mut Node>| {
                let Some(ghost) = dragging.ghost else {
                    return;
                };
                let Ok(mut node) = nodes.get_mut(ghost) else {
                    return;
                };
                let at = drag.pointer_location.position / scale.0
                    + GHOST_OFFSET;

                node.left = px(at.x);
                node.top = px(at.y);
            },
        )
        .observe(
            move |_: On<Pointer<DragEnd>>,
                  mut dragging: ResMut<Dragging>,
                  mut commands: Commands| {
                if let Some(ghost) = dragging.ghost.take() {
                    commands.entity(ghost).despawn();
                }
                dragging.subject = None;
                dragging.target = None;
            },
        )
}

/// Makes `elem` somewhere a row can be dropped, landing it at `at`
/// relative to `subject`.
pub(super) fn drops<'r, 'u, 'a, E: Element<BevyHost>>(
    elem: &'r mut ElementMut<'u, 'a, BevyHost, E>,
    subject: Entity,
    at: At,
) -> &'r mut ElementMut<'u, 'a, BevyHost, E> {
    elem.observe(
        move |over: On<Pointer<DragOver>>,
              mut dragging: ResMut<Dragging>| {
            if over.button == PointerButton::Primary {
                dragging.target = Some((subject, at));
            }
        },
    )
    .observe(
        move |_: On<Pointer<DragLeave>>,
              mut dragging: ResMut<Dragging>| {
            if dragging.target == Some((subject, at)) {
                dragging.target = None;
            }
        },
    )
    .observe(
        move |drop: On<Pointer<DragDrop>>,
              dragging: Res<Dragging>,
              mut commands: Commands| {
            // Read rather than taken: the drop lands before the drag
            // ends, which is what clears it.
            let Some(dragged) = dragging.subject else {
                return;
            };
            if drop.button != PointerButton::Primary {
                return;
            }

            commands.queue(move |world: &mut World| {
                apply(world, dragged, subject, at);
            });
        },
    )
}

/// What follows the cursor while a row is being dragged.
///
/// [`Pickable::IGNORE`] because it sits directly under the cursor: seen
/// by the pointer it would be the only thing ever dragged over, and no
/// row would light up.
fn ghost(at: Vec2, name: String, theme: &EditorTheme) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(at.x + GHOST_OFFSET.x),
            top: px(at.y + GHOST_OFFSET.y),
            padding: UiRect::axes(px(6), px(2)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        },
        BackgroundColor(theme.accent.with_alpha(0.85)),
        GlobalZIndex(200),
        Pickable::IGNORE,
        children![(
            Text::new(name),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(theme.palette.base[0]),
            TextLayout::linebreak(LineBreak::NoWrap),
            Pickable::IGNORE,
        )],
    )
}

/// Moves `dragged` to where `at` puts it relative to `row`.
///
/// One [`insert_child`](EntityWorldMut::insert_child) does either job:
/// handed a child the parent already holds it moves it within the
/// list, and handed one it does not it takes it from wherever it was.
fn apply(world: &mut World, dragged: Entity, row: Entity, at: At) {
    let Some((parent, index)) = destination(world, dragged, row, at)
    else {
        return;
    };

    // Read before the move, since attaching is what changes them.
    let local = world
        .get::<GlobalTransform>(dragged)
        .zip(world.get::<GlobalTransform>(parent))
        .map(|(dragged, parent)| dragged.reparented_to(parent));

    world.entity_mut(parent).insert_child(index, dragged);

    if let Some(local) = local {
        world.entity_mut(dragged).insert(local);
    }
}

/// The parent to put `dragged` under and where in its children, or
/// `None` for a drop that would not hold: onto itself, or into its own
/// subtree, which is a parent of its own parent.
fn destination(
    world: &World,
    dragged: Entity,
    row: Entity,
    at: At,
) -> Option<(Entity, usize)> {
    if dragged == row || holds(world, dragged, row) {
        return None;
    }

    let (parent, index) = match at {
        // Past the end, for a place at the end of the list.
        At::Into => (row, usize::MAX),
        _ => {
            let parent = world.get::<ChildOf>(row)?.parent();
            let index = world
                .get::<Children>(parent)?
                .iter()
                .position(|child| child == row)?;

            match at {
                At::After => (parent, index + 1),
                _ => (parent, index),
            }
        }
    };

    Some((parent, settle(world, parent, dragged, index)))
}

/// `index` as the list will actually be when the entity is put back.
///
/// Moving within one parent takes the entity out before putting it
/// back, so every slot after it shifts left. A forward move
/// overshoots by one; asking for the end overshoots the shortened
/// list entirely and panics instead of clamping.
fn settle(
    world: &World,
    parent: Entity,
    dragged: Entity,
    index: usize,
) -> usize {
    let Some(children) = world.get::<Children>(parent) else {
        return 0;
    };
    // Arriving from another parent is added first and only then
    // placed, so the list it lands in is never short.
    let Some(current) =
        children.iter().position(|child| child == dragged)
    else {
        return index;
    };

    let shifted = if index > current { index - 1 } else { index };

    shifted.min(children.len() - 1)
}

/// Whether `entity` is somewhere above `row`.
fn holds(world: &World, entity: Entity, row: Entity) -> bool {
    let mut above = world.get::<ChildOf>(row).map(ChildOf::parent);

    while let Some(parent) = above {
        if parent == entity {
            return true;
        }
        above = world.get::<ChildOf>(parent).map(ChildOf::parent);
    }

    false
}
