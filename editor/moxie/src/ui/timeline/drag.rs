//! Resizing an action's duration, and moving a node's body: a delay
//! edit inside `All`/`Flow`/at a block's own position, or a reorder
//! among siblings inside a `Chain`.
//!
//! Nothing here writes `EditorScene` until the drag ends - a `Pointer
//! <Drag>` write would trigger the timeline's own
//! `value_changed(block_view)` watch, rebuilding (despawning and
//! respawning) every box on the timeline mid-drag, including the one
//! being dragged, which ends the gesture after a single event. A live
//! preview instead clones the tree, applies the drag's edit to the
//! clone, and lays *that* out - the same real `block_layout::layout`,
//! so a resize's cascade onto later `Chain` siblings or a reorder's
//! shuffle come out correct by construction rather than by a
//! hand-rolled approximation. [`BoxPath`] is how the result finds its
//! way back onto the real, already-spawned boxes.

use core::time::Duration;
use std::collections::BTreeSet;

use bevy::picking::events::{Drag, DragEnd, DragStart, Pointer};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::{Node as UiNode, UiScale};
use bevy_fynix::EntityExt as _;
use bevy_motiongfx::scene::backend::Backend;
use fynix_mock::element::Element;
use fynix_mock::ui::ElementMut;
use motiongfx_scene::block::{Block, Combinator, Node};
use moxie_ui::reactive::BevyHost;

use super::super::action::{node_at, node_at_mut};
use super::BlockFoldState;
use crate::block_layout::{self, Placed};
use crate::{EditorScene, seconds_for};

/// Never zero or negative: a leaf with no duration has nothing to
/// play.
const MIN_DURATION: f32 = 0.05;

/// A resize or move in progress. `None` when nothing is being
/// dragged.
#[derive(Resource, Default)]
pub(in crate::ui) struct Dragging(Option<Kind>);

enum Kind {
    /// Its duration as the drag started.
    Resize { start: Duration },
    /// Its delay as the drag started.
    Delay { start: Duration },
    /// Reordering inside a `Chain`: each sibling's `(x, width)` as the
    /// drag started, in child order, so the live target index comes
    /// from where the cursor's ended up among them without
    /// re-measuring the tree on every move.
    Reorder {
        siblings: Vec<(f32, f32)>,
        index: usize,
    },
}

/// Which node in the tree a box renders, so a live preview can find
/// and reposition it directly - the same reason [`Dragging`] avoids
/// `EditorScene`, applied to every other box a drag might shift too.
#[derive(Component)]
pub(super) struct BoxPath(pub(super) Vec<usize>);

/// The block holding the node at `path`, and its own index in it.
/// `None` only for the root's own path, which nothing holds.
fn parent_of<'a>(
    root: &'a Block<Backend>,
    path: &[usize],
) -> Option<(&'a Block<Backend>, usize)> {
    let (&index, parent_path) = path.split_last()?;
    let parent = if parent_path.is_empty() {
        root
    } else {
        match node_at(root, parent_path)? {
            Node::Block { block, .. } => block,
            _ => return None,
        }
    };
    Some((parent, index))
}

fn parent_of_mut<'a>(
    root: &'a mut Block<Backend>,
    path: &[usize],
) -> Option<(&'a mut Block<Backend>, usize)> {
    let (&index, parent_path) = path.split_last()?;
    let parent = if parent_path.is_empty() {
        root
    } else {
        match node_at_mut(root, parent_path)? {
            Node::Block { block, .. } => block,
            _ => return None,
        }
    };
    Some((parent, index))
}

fn node_delay(node: &Node<Backend>) -> Duration {
    match node {
        Node::Block { delay, .. }
        | Node::Action { delay, .. }
        | Node::Draft { delay, .. } => delay.unwrap_or_default(),
    }
}

fn set_node_delay(node: &mut Node<Backend>, value: Duration) {
    let delay = match node {
        Node::Block { delay, .. }
        | Node::Action { delay, .. }
        | Node::Draft { delay, .. } => delay,
    };
    *delay = Some(value);
}

/// A leaf's own duration - `None` for a block, which has none of its
/// own to resize.
fn node_duration(node: &Node<Backend>) -> Option<Duration> {
    match node {
        Node::Action { action, .. } => Some(action.duration),
        Node::Draft { duration, .. } => Some(*duration),
        Node::Block { .. } => None,
    }
}

fn set_node_duration(node: &mut Node<Backend>, value: Duration) {
    match node {
        Node::Action { action, .. } => action.duration = value,
        Node::Draft { duration, .. } => *duration = value,
        Node::Block { .. } => {}
    }
}

/// A clone of the real tree with `edit` applied, laid out exactly as
/// `EditorScene` itself would be, and pushed onto every already-
/// spawned box that a [`BoxPath`] says the layout still has an entry
/// for - never written back to `EditorScene` itself.
fn preview(
    world: &mut World,
    edit: impl FnOnce(&mut Block<Backend>),
) {
    let Some(editor_scene) = world.get_resource::<EditorScene>()
    else {
        return;
    };
    let mut tree = editor_scene.scene().0.animation.clone();
    edit(&mut tree);

    let empty = BTreeSet::new();
    let folded = world
        .get_resource::<BlockFoldState>()
        .map_or(&empty, |state| &state.0);
    let placements = block_layout::layout(&tree, folded);

    let mut boxes = world.query::<(&BoxPath, &mut UiNode)>();
    for (at, mut node) in boxes.iter_mut(world) {
        let Some(placed) =
            placements.iter().find(|placed| placed.path == at.0)
        else {
            continue;
        };
        node.left = px(placed.x);
        node.top = px(placed.y);
        node.width = px(placed.w);
        node.height = px(placed.h);
    }
}

/// Wires `handle`, a small box overlaid on a leaf's (action or
/// draft's) right edge, to resize that leaf's `duration`.
pub(super) fn resizes<E: Element<BevyHost>>(
    handle: &mut ElementMut<BevyHost, E>,
    path: Vec<usize>,
) {
    handle
        .observe({
            let path = path.clone();
            move |start: On<Pointer<DragStart>>,
                  editor_scene: Res<EditorScene>,
                  mut dragging: ResMut<Dragging>| {
                if start.button != PointerButton::Primary {
                    return;
                }
                let Some(start) =
                    node_at(&editor_scene.scene().0.animation, &path)
                        .and_then(node_duration)
                else {
                    return;
                };
                dragging.0 = Some(Kind::Resize { start });
            }
        })
        .observe({
            let path = path.clone();
            move |drag: On<Pointer<Drag>>,
                  scale: Res<UiScale>,
                  dragging: Res<Dragging>,
                  mut commands: Commands| {
                if drag.button != PointerButton::Primary {
                    return;
                }
                let Some(Kind::Resize { start }) = &dragging.0 else {
                    return;
                };
                let seconds = (start.as_secs_f32()
                    + seconds_for(drag.distance.x / scale.0))
                .max(MIN_DURATION);
                let path = path.clone();

                commands.queue(move |world: &mut World| {
                    preview(world, |tree| {
                        if let Some(node) = node_at_mut(tree, &path) {
                            set_node_duration(
                                node,
                                Duration::from_secs_f32(seconds),
                            );
                        }
                    });
                });
            }
        })
        .observe({
            let path = path.clone();
            move |end: On<Pointer<DragEnd>>,
                  scale: Res<UiScale>,
                  mut dragging: ResMut<Dragging>,
                  mut commands: Commands| {
                let Some(Kind::Resize { start }) = dragging.0.take()
                else {
                    return;
                };
                if end.button != PointerButton::Primary {
                    return;
                }
                let seconds = (start.as_secs_f32()
                    + seconds_for(end.distance.x / scale.0))
                .max(MIN_DURATION);
                let path = path.clone();

                commands.queue(move |world: &mut World| {
                    let Some(mut editor) =
                        world.get_resource_mut::<EditorScene>()
                    else {
                        return;
                    };
                    if let Some(node) = node_at_mut(
                        &mut editor.edit().0.animation,
                        &path,
                    ) {
                        set_node_duration(
                            node,
                            Duration::from_secs_f32(seconds),
                        );
                    }
                });
            }
        });
}

/// Wires `elem`, a node's own box (action or block alike), to move
/// its body: a `delay` edit, or - inside a `Chain` - a reorder among
/// its siblings. Both preview live and commit to `EditorScene` only
/// on drop.
pub(super) fn moves<E: Element<BevyHost>>(
    elem: &mut ElementMut<BevyHost, E>,
    path: Vec<usize>,
    placements: Vec<Placed>,
) {
    elem.observe({
        let path = path.clone();
        move |start: On<Pointer<DragStart>>,
              editor_scene: Res<EditorScene>,
              mut dragging: ResMut<Dragging>| {
            if start.button != PointerButton::Primary {
                return;
            }
            let root = &editor_scene.scene().0.animation;
            let Some((parent, index)) = parent_of(root, &path) else {
                return;
            };

            dragging.0 =
                Some(if parent.combinator == Combinator::Chain {
                    let parent_path = &path[..path.len() - 1];
                    let siblings = (0..parent.children.len())
                        .map(|i| {
                            let mut child = parent_path.to_vec();
                            child.push(i);
                            placements
                                .iter()
                                .find(|p| p.path == child)
                                .map_or((0.0, 0.0), |p| (p.x, p.w))
                        })
                        .collect();
                    Kind::Reorder { siblings, index }
                } else {
                    let start = node_at(root, &path)
                        .map(node_delay)
                        .unwrap_or_default();
                    Kind::Delay { start }
                });
        }
    })
    .observe({
        let path = path.clone();
        move |drag: On<Pointer<Drag>>,
              scale: Res<UiScale>,
              dragging: Res<Dragging>,
              mut commands: Commands| {
            if drag.button != PointerButton::Primary {
                return;
            }

            match &dragging.0 {
                Some(Kind::Delay { start }) => {
                    let seconds = (start.as_secs_f32()
                        + seconds_for(drag.distance.x / scale.0))
                    .max(0.0);
                    let path = path.clone();

                    commands.queue(move |world: &mut World| {
                        preview(world, |tree| {
                            if let Some(node) =
                                node_at_mut(tree, &path)
                            {
                                set_node_delay(
                                    node,
                                    Duration::from_secs_f32(seconds),
                                );
                            }
                        });
                    });
                }
                Some(Kind::Reorder { siblings, index }) => {
                    let target = reorder_target(
                        siblings,
                        *index,
                        drag.distance.x / scale.0,
                    );
                    let path = path.clone();

                    commands.queue(move |world: &mut World| {
                        preview(world, |tree| {
                            move_child(tree, &path, target);
                        });
                    });
                }
                Some(Kind::Resize { .. }) | None => {}
            }
        }
    })
    .observe({
        let path = path.clone();
        move |end: On<Pointer<DragEnd>>,
              scale: Res<UiScale>,
              mut dragging: ResMut<Dragging>,
              mut commands: Commands| {
            if end.button != PointerButton::Primary {
                dragging.0 = None;
                return;
            }

            match dragging.0.take() {
                Some(Kind::Delay { start }) => {
                    let seconds = (start.as_secs_f32()
                        + seconds_for(end.distance.x / scale.0))
                    .max(0.0);
                    let path = path.clone();

                    commands.queue(move |world: &mut World| {
                        let Some(mut editor) =
                            world.get_resource_mut::<EditorScene>()
                        else {
                            return;
                        };
                        if let Some(node) = node_at_mut(
                            &mut editor.edit().0.animation,
                            &path,
                        ) {
                            set_node_delay(
                                node,
                                Duration::from_secs_f32(seconds),
                            );
                        }
                    });
                }
                Some(Kind::Reorder { siblings, index }) => {
                    let target = reorder_target(
                        &siblings,
                        index,
                        end.distance.x / scale.0,
                    );
                    if target != index {
                        let path = path.clone();
                        commands.queue(move |world: &mut World| {
                            let Some(mut editor) = world
                                .get_resource_mut::<EditorScene>()
                            else {
                                return;
                            };
                            move_child(
                                &mut editor.edit().0.animation,
                                &path,
                                target,
                            );
                        });
                    }
                }
                Some(Kind::Resize { .. }) | None => {}
            }
        }
    });
}

/// Where the dragged sibling belongs among `siblings` now that the
/// drag has moved it `dx` from its own starting `x` - a count of how
/// many others it's now past.
fn reorder_target(
    siblings: &[(f32, f32)],
    index: usize,
    dx: f32,
) -> usize {
    let (x, w) = siblings[index];
    let center = x + w / 2.0 + dx;

    siblings
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != index)
        .filter(|&(_, &(x, w))| x + w / 2.0 < center)
        .count()
}

/// Moves the node at `path` to `target` among its own siblings.
fn move_child(
    root: &mut Block<Backend>,
    path: &[usize],
    target: usize,
) {
    let Some((parent, index)) = parent_of_mut(root, path) else {
        return;
    };
    if target == index {
        return;
    }

    let node = parent.children.remove(index);
    let target = target.min(parent.children.len());
    parent.children.insert(target, node);
}
