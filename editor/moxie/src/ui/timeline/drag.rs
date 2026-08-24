//! Resizing an action's duration, and moving a node's body: a delay
//! edit inside `All`/`Flow`/at a block's own position, or a reorder
//! among siblings inside a `Chain`.
//!
//! No ghost, unlike `hierarchy/drag.rs`'s row drag: a resize or a
//! delay edit already redraws the real box live as it's dragged, so a
//! second floating copy would be redundant. A `Chain` reorder is the
//! one case with nothing to show live - it commits on drop, from the
//! drag's total distance against where its siblings started.

use core::time::Duration;

use bevy::picking::events::{Drag, DragEnd, DragStart, Pointer};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::UiScale;
use bevy_fynix::EntityExt as _;
use bevy_motiongfx::scene::backend::Backend;
use fynix_mock::element::Element;
use fynix_mock::ui::ElementMut;
use motiongfx_scene::block::{Block, Combinator, Node};
use moxie_ui::reactive::BevyHost;

use super::super::action::{node_at, node_at_mut};
use crate::block_layout::Placed;
use crate::{EditorScene, seconds_for};

/// Never zero or negative: an action with no duration has nothing to
/// play.
const MIN_DURATION: f32 = 0.05;

/// A resize or move in progress. `None` when nothing is being
/// dragged.
#[derive(Resource, Default)]
pub(in crate::ui) struct Dragging(Option<Kind>);

enum Kind {
    /// A leaf's own duration (action or draft), before the drag
    /// started.
    Resize { start: Duration },
    /// A node's own delay, before the drag started.
    Delay { start: Duration },
    /// Reordering inside a `Chain`: each sibling's `(x, width)` as the
    /// drag started, in child order, so the drop's target index comes
    /// from where the cursor ends up among them without re-measuring
    /// the tree on every move.
    Reorder {
        path: Vec<usize>,
        siblings: Vec<(f32, f32)>,
        index: usize,
    },
}

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

/// Wires `elem`, a leaf's own box (action or draft), to resize its
/// `duration` by dragging its right edge.
pub(super) fn resizes<E: Element<BevyHost>>(
    elem: &mut ElementMut<BevyHost, E>,
    path: Vec<usize>,
) {
    elem.observe({
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
              mut editor_scene: ResMut<EditorScene>| {
            if drag.button != PointerButton::Primary {
                return;
            }
            let Some(Kind::Resize { start }) = &dragging.0 else {
                return;
            };
            let seconds = (start.as_secs_f32()
                + seconds_for(drag.distance.x / scale.0))
            .max(MIN_DURATION);

            if let Some(node) = node_at_mut(
                &mut editor_scene.edit().0.animation,
                &path,
            ) {
                set_node_duration(
                    node,
                    Duration::from_secs_f32(seconds),
                );
            }
        }
    })
    .observe(
        |_: On<Pointer<DragEnd>>, mut dragging: ResMut<Dragging>| {
            dragging.0 = None;
        },
    );
}

/// Wires `elem`, a node's own box (action or block alike), to move
/// its body: a live `delay` edit, or - inside a `Chain` - a reorder
/// among its siblings, committed on drop.
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
                    Kind::Reorder {
                        path: path.clone(),
                        siblings,
                        index,
                    }
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
              mut editor_scene: ResMut<EditorScene>| {
            if drag.button != PointerButton::Primary {
                return;
            }
            let Some(Kind::Delay { start }) = &dragging.0 else {
                return;
            };
            let seconds = (start.as_secs_f32()
                + seconds_for(drag.distance.x / scale.0))
            .max(0.0);

            if let Some(node) = node_at_mut(
                &mut editor_scene.edit().0.animation,
                &path,
            ) {
                set_node_delay(
                    node,
                    Duration::from_secs_f32(seconds),
                );
            }
        }
    })
    .observe({
        move |end: On<Pointer<DragEnd>>,
              scale: Res<UiScale>,
              mut dragging: ResMut<Dragging>,
              mut commands: Commands| {
            if end.button != PointerButton::Primary {
                dragging.0 = None;
                return;
            }
            let Some(Kind::Reorder {
                path,
                siblings,
                index,
            }) = dragging.0.take()
            else {
                return;
            };

            let (x, w) = siblings[index];
            let center = x + w / 2.0 + end.distance.x / scale.0;
            let target = siblings
                .iter()
                .enumerate()
                .filter(|&(i, _)| i != index)
                .filter(|&(_, &(x, w))| x + w / 2.0 < center)
                .count();

            if target != index {
                commands.queue(move |world: &mut World| {
                    reorder(world, &path, target);
                });
            }
        }
    });
}

fn reorder(world: &mut World, path: &[usize], target: usize) {
    let Some(mut editor) = world.get_resource_mut::<EditorScene>()
    else {
        return;
    };
    let Some((parent, index)) =
        parent_of_mut(&mut editor.edit().0.animation, path)
    else {
        return;
    };

    let node = parent.children.remove(index);
    let target = target.min(parent.children.len());
    parent.children.insert(target, node);
}
