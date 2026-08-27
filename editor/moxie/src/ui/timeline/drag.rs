//! Moving and resizing a node's box by dragging one of its edges.
//!
//! The left edge always edits `delay`, the right edge (an action or
//! draft leaf only - a block has no `duration` of its own) always
//! edits `duration`. Splitting the two into dedicated handles, rather
//! than reading a body-drag's direction, leaves the box's body free
//! for a future drag-to-merge/chain gesture without the two ever
//! fighting over what a drag into another node's territory means.
//!
//! There's no stored position to drag, either way - the only knob
//! either edit actually turns is `delay` or `duration`, everything
//! else in `block_layout.rs` derives from those. Nothing writes
//! [`EditorScene`] until [`DragEnd`]: `TrackViewport`'s box list
//! watches it, so a write on every `Pointer<Drag>` would rebuild the
//! very box being dragged out from under the gesture after one
//! frame. Instead, each `Pointer<Drag>` lays out a scratch copy of
//! the tree with the tentative edit applied and pushes the result
//! straight onto the already-spawned box entities via [`BoxPath`],
//! without touching the composer tree that owns them. Escape cancels,
//! laying the untouched tree back out to undo the preview.

use core::time::Duration;

use bevy::feathers::cursor::EntityCursor;
use bevy::input::ButtonInput;
use bevy::picking::events::{Drag, DragEnd, DragStart, Pointer};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::UiScale;
use bevy::window::SystemCursorIcon;
use bevy_fynix::EntityExt;
use bevy_motiongfx::scene::backend::Backend;
use fynix_mock::element::Element;
use fynix_mock::ui::ElementMut;
use motiongfx_scene::block::Node as SceneNode;
use moxie_ui::reactive::BevyHost;

use super::super::action::{node_at, node_at_mut};
use super::BlockFoldState;
use crate::block_layout;
use crate::{EditorScene, PIXELS_PER_SECOND};

/// How wide an edge handle is.
pub(crate) const EDGE_HANDLE_PX: f32 = 6.0;

/// Never resized shorter than this.
const MIN_DURATION: Duration = Duration::from_millis(50);

/// What an edge handle edits.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    /// The left edge: `delay`.
    Move,
    /// The right edge: `duration`.
    Resize,
}

/// The node being dragged, if any.
#[derive(Resource, Default)]
pub(crate) struct Dragging(Option<Gesture>);

/// One drag in progress: which node it edits, and what it would
/// commit if released right now.
struct Gesture {
    path: Vec<usize>,
    kind: Kind,
    cursor_start: Vec2,
    /// `delay` (move) or `duration` (resize) before the drag started.
    base_secs: f32,
    /// The same, live - what release would commit.
    value_secs: f32,
}

/// Marks a box entity ([`TimelineAction`](moxie_ui::elements::TimelineAction)
/// or [`TimelineBlock`](moxie_ui::elements::TimelineBlock)) with the
/// path it was built for, so a live drag can retarget it by path
/// without going through the composer tree that owns it.
#[derive(Component, Clone)]
pub(crate) struct BoxPath(pub(crate) Vec<usize>);

/// A path's box and its gap are two different entities, each needing
/// its own retargeting, so this is its own component rather than
/// [`BoxPath`] shared with [`TimelineGap`](moxie_ui::elements::TimelineGap).
#[derive(Component, Clone)]
pub(crate) struct GapPath(pub(crate) Vec<usize>);

/// Makes `handle` an edge: dragging it edits `path`'s `delay`
/// (`Kind::Move`) or `duration` (`Kind::Resize`).
pub(crate) fn edge<'r, 'u, 'a, E: Element<BevyHost>>(
    handle: &'r mut ElementMut<'u, 'a, BevyHost, E>,
    path: Vec<usize>,
    kind: Kind,
) -> &'r mut ElementMut<'u, 'a, BevyHost, E> {
    handle
        .insert(EntityCursor::System(SystemCursorIcon::EwResize))
        .observe(
            move |start: On<Pointer<DragStart>>,
                  scale: Res<UiScale>,
                  editor_scene: Res<EditorScene>,
                  mut dragging: ResMut<Dragging>| {
                if start.button != PointerButton::Primary {
                    return;
                }
                let Some(base_secs) =
                    base_seconds(&editor_scene, &path, kind)
                else {
                    return;
                };

                dragging.0 = Some(Gesture {
                    path: path.clone(),
                    kind,
                    cursor_start: start.pointer_location.position
                        / scale.0,
                    base_secs,
                    value_secs: base_secs,
                });
            },
        )
        .observe(
            move |drag: On<Pointer<Drag>>,
                  scale: Res<UiScale>,
                  mut dragging: ResMut<Dragging>,
                  editor_scene: Res<EditorScene>,
                  folded: Res<BlockFoldState>,
                  boxes: Query<(&BoxPath, &mut Node)>,
                  gaps: Query<
                (&GapPath, &mut Node),
                Without<BoxPath>,
            >| {
                let Some(gesture) = &mut dragging.0 else {
                    return;
                };
                let cursor = drag.pointer_location.position / scale.0;
                let dx_secs = (cursor.x - gesture.cursor_start.x)
                    / PIXELS_PER_SECOND;

                gesture.value_secs = match gesture.kind {
                    Kind::Move => {
                        (gesture.base_secs + dx_secs).max(0.0)
                    }
                    Kind::Resize => (gesture.base_secs + dx_secs)
                        .max(MIN_DURATION.as_secs_f32()),
                };

                relayout(
                    &editor_scene,
                    &folded,
                    &gesture.path,
                    gesture.kind,
                    gesture.value_secs,
                    boxes,
                    gaps,
                );
            },
        )
        .observe(
            move |_: On<Pointer<DragEnd>>,
                  mut dragging: ResMut<Dragging>,
                  mut commands: Commands| {
                let Some(gesture) = dragging.0.take() else {
                    return;
                };
                if gesture.value_secs != gesture.base_secs {
                    commands.queue(move |world: &mut World| {
                        commit(
                            world,
                            &gesture.path,
                            gesture.kind,
                            gesture.value_secs,
                        );
                    });
                }
            },
        )
}

/// Drops whatever's being dragged without committing it, laying the
/// untouched tree back out to undo whatever the drag previewed.
pub(crate) fn cancel_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut dragging: ResMut<Dragging>,
    editor_scene: Res<EditorScene>,
    folded: Res<BlockFoldState>,
    boxes: Query<(&BoxPath, &mut Node)>,
    gaps: Query<(&GapPath, &mut Node), Without<BoxPath>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    let Some(gesture) = dragging.0.take() else {
        return;
    };
    relayout(
        &editor_scene,
        &folded,
        &gesture.path,
        gesture.kind,
        gesture.base_secs,
        boxes,
        gaps,
    );
}

/// Lays `path`'s tree back out with `secs` applied to `kind`'s edit,
/// and pushes the result onto whichever spawned box and gap entities
/// [`BoxPath`]/[`GapPath`] match - a scratch copy, so nothing here
/// ever touches the real [`EditorScene`].
fn relayout(
    editor_scene: &EditorScene,
    folded: &BlockFoldState,
    path: &[usize],
    kind: Kind,
    secs: f32,
    mut boxes: Query<(&BoxPath, &mut Node)>,
    mut gaps: Query<(&GapPath, &mut Node), Without<BoxPath>>,
) {
    let mut animation = editor_scene.scene().0.animation.clone();
    let Some(node) = node_at_mut(&mut animation, path) else {
        return;
    };
    apply_edit(node, kind, secs);

    for placed in block_layout::layout(&animation, folded.paths()) {
        for (box_path, mut node) in &mut boxes {
            if box_path.0 != placed.path {
                continue;
            }
            node.left = px(placed.x);
            node.top = px(placed.y);
            node.width = px(placed.w);
            node.height = px(placed.h);
            break;
        }

        for (gap_path, mut node) in &mut gaps {
            if gap_path.0 != placed.path {
                continue;
            }
            // A gap already spawned for this path, but the drag has
            // since closed it, collapses to nothing rather than
            // showing a stale width - a fresh gap the drag opens up
            // where none existed has to wait for the next real
            // rebuild, same as the edge handles and fold chevron.
            let width = placed.gap_x.map_or(0.0, |gap_x| {
                node.left = px(gap_x);
                node.top = px(placed.y);
                node.height = px(placed.h);
                placed.x - gap_x
            });
            node.width = px(width);
            break;
        }
    }
}

/// `path`'s current `delay` (move) or `duration` (resize) - `None`
/// for a resize on a block, which has no duration of its own to
/// grab, or a path a concurrent edit has since made dangling.
fn base_seconds(
    editor_scene: &EditorScene,
    path: &[usize],
    kind: Kind,
) -> Option<f32> {
    let node = node_at(&editor_scene.scene().0.animation, path)?;
    match kind {
        Kind::Move => Some(delay_secs(node)),
        Kind::Resize => duration_secs(node),
    }
}

fn delay_secs(node: &SceneNode<Backend>) -> f32 {
    match node {
        SceneNode::Block { delay, .. }
        | SceneNode::Action { delay, .. }
        | SceneNode::Draft { delay, .. } => {
            delay.unwrap_or_default().as_secs_f32()
        }
    }
}

fn duration_secs(node: &SceneNode<Backend>) -> Option<f32> {
    match node {
        SceneNode::Action { action, .. } => {
            Some(action.duration.as_secs_f32())
        }
        SceneNode::Draft { duration, .. } => {
            Some(duration.as_secs_f32())
        }
        SceneNode::Block { .. } => None,
    }
}

/// Writes the drag's result back into the scene: `path`'s `delay` for
/// a move, its `duration` for a resize.
fn commit(world: &mut World, path: &[usize], kind: Kind, secs: f32) {
    let Some(mut editor_scene) =
        world.get_resource_mut::<EditorScene>()
    else {
        return;
    };
    let Some(node) =
        node_at_mut(&mut editor_scene.edit().0.animation, path)
    else {
        return;
    };
    apply_edit(node, kind, secs);
}

/// `kind`'s edit, applied in place to whichever field it names.
fn apply_edit(node: &mut SceneNode<Backend>, kind: Kind, secs: f32) {
    match kind {
        Kind::Move => {
            let delay = match node {
                SceneNode::Block { delay, .. }
                | SceneNode::Action { delay, .. }
                | SceneNode::Draft { delay, .. } => delay,
            };
            *delay =
                (secs > 0.0).then(|| Duration::from_secs_f32(secs));
        }
        Kind::Resize => match node {
            SceneNode::Action { action, .. } => {
                action.duration = Duration::from_secs_f32(secs);
            }
            SceneNode::Draft { duration, .. } => {
                *duration = Duration::from_secs_f32(secs);
            }
            SceneNode::Block { .. } => {}
        },
    }
}
