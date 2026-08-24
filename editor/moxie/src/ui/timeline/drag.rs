//! Resizing an action's duration, and moving a node's body: a delay
//! edit inside `All`/`Flow`/at a block's own position, or a reorder
//! among siblings inside a `Chain`.
//!
//! A resize or a delay edit writes `EditorScene` only once, on drop -
//! not on every `Pointer<Drag>`. Live feedback comes from poking the
//! dragged box's own `Node` directly instead: any earlier write
//! landed on `EditorScene` would trigger the timeline's own
//! `value_changed(block_view)` watch, rebuilding (despawning and
//! respawning) every box on the timeline mid-drag - including the one
//! being dragged, ending the gesture after a single event. A `Chain`
//! reorder was never writing mid-drag to begin with; nothing here
//! shows it live either, both commit only on drop.

use core::time::Duration;

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
use crate::block_layout::Placed;
use crate::{EditorScene, px_for, seconds_for};

/// Never zero or negative: a leaf with no duration has nothing to
/// play.
const MIN_DURATION: f32 = 0.05;

/// A resize or move in progress. `None` when nothing is being
/// dragged.
#[derive(Resource, Default)]
pub(in crate::ui) struct Dragging(Option<Kind>);

enum Kind {
    /// The box's own entity and width, plus its duration, all as the
    /// drag started - so a live update never has to read `Node` back
    /// (`UiScale` could have changed it) or reach into `EditorScene`.
    Resize {
        entity: Entity,
        start_width: f32,
        start_duration: Duration,
    },
    /// The box's own entity and left edge, plus its delay, all as the
    /// drag started.
    Delay {
        entity: Entity,
        start_left: f32,
        start_delay: Duration,
    },
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

/// Wires `handle`, a small box overlaid on a leaf's (action or
/// draft's) right edge, to resize that leaf's `duration`. `entity` is
/// the leaf's own box, not `handle` itself - dragging the handle
/// resizes the box it sits on, not the handle.
pub(super) fn resizes<E: Element<BevyHost>>(
    handle: &mut ElementMut<BevyHost, E>,
    path: Vec<usize>,
    entity: Entity,
    start_width: f32,
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
                let Some(start_duration) =
                    node_at(&editor_scene.scene().0.animation, &path)
                        .and_then(node_duration)
                else {
                    return;
                };
                dragging.0 = Some(Kind::Resize {
                    entity,
                    start_width,
                    start_duration,
                });
            }
        })
        .observe(
            move |drag: On<Pointer<Drag>>,
                  scale: Res<UiScale>,
                  dragging: Res<Dragging>,
                  mut nodes: Query<&mut UiNode>| {
                if drag.button != PointerButton::Primary {
                    return;
                }
                let Some(Kind::Resize {
                    entity,
                    start_width,
                    ..
                }) = &dragging.0
                else {
                    return;
                };
                let width = (start_width + drag.distance.x / scale.0)
                    .max(px_for(Duration::from_secs_f32(
                        MIN_DURATION,
                    )));

                if let Ok(mut node) = nodes.get_mut(*entity) {
                    node.width = px(width);
                }
            },
        )
        .observe({
            let path = path.clone();
            move |end: On<Pointer<DragEnd>>,
                  scale: Res<UiScale>,
                  mut dragging: ResMut<Dragging>,
                  mut commands: Commands| {
                let Some(Kind::Resize { start_duration, .. }) =
                    dragging.0.take()
                else {
                    return;
                };
                if end.button != PointerButton::Primary {
                    return;
                }
                let seconds = (start_duration.as_secs_f32()
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
/// its body: a live `delay` edit, or - inside a `Chain` - a reorder
/// among its siblings, both committed on drop.
pub(super) fn moves<E: Element<BevyHost>>(
    elem: &mut ElementMut<BevyHost, E>,
    path: Vec<usize>,
    start_left: f32,
    placements: Vec<Placed>,
) {
    let entity = elem.id();

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
                    let start_delay = node_at(root, &path)
                        .map(node_delay)
                        .unwrap_or_default();
                    Kind::Delay {
                        entity,
                        start_left,
                        start_delay,
                    }
                });
        }
    })
    .observe(
        move |drag: On<Pointer<Drag>>,
              scale: Res<UiScale>,
              dragging: Res<Dragging>,
              mut nodes: Query<&mut UiNode>| {
            if drag.button != PointerButton::Primary {
                return;
            }
            let Some(Kind::Delay {
                entity, start_left, ..
            }) = &dragging.0
            else {
                return;
            };
            let left =
                (start_left + drag.distance.x / scale.0).max(0.0);

            if let Ok(mut node) = nodes.get_mut(*entity) {
                node.left = px(left);
            }
        },
    )
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
                Some(Kind::Delay { start_delay, .. }) => {
                    let seconds = (start_delay.as_secs_f32()
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
                Some(Kind::Reorder {
                    path,
                    siblings,
                    index,
                }) => {
                    let (x, w) = siblings[index];
                    let center =
                        x + w / 2.0 + end.distance.x / scale.0;
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
                _ => {}
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
