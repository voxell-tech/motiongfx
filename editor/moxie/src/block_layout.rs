//! Lays a scene's [`Block`] tree out as nested boxes, not one flat
//! row per node. A block is a bordered container spanning its time
//! range, holding its children as filled bars (actions) or nested
//! containers (blocks).
//!
//! Horizontal position always comes straight from a node's resolved
//! start time ([`crate::px_for`]). Nesting only affects the vertical
//! axis: a block's box literally encloses its children's boxes,
//! rather than implying the relationship through indentation.

use core::time::Duration;
use std::collections::BTreeSet;

use bevy_motiongfx::scene::backend::Backend;
use motiongfx_scene::block::{Block, Combinator, Node};

/// Height of an action leaf's bar, and of a block's header strip.
const ROW_HEIGHT: f32 = 20.0;
const HEADER_HEIGHT: f32 = 20.0;
/// Vertical gap between lanes that would otherwise overlap in time.
const LANE_GAP: f32 = 4.0;
const MIN_WIDTH: f32 = 2.0;

/// One box to draw: a block header (its name, or its combinator if
/// unnamed) or an action leaf.
#[derive(Clone, PartialEq)]
pub(crate) struct Placed {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) depth: usize,
    /// `Some` for a block header box; `None` for an action leaf.
    pub(crate) label: Option<String>,
    /// An action leaf's own name, if set. `None` for a block - its
    /// name, if any, is already folded into `label`.
    pub(crate) name: Option<String>,
    /// `true` when a block's children are folded away. Always `false`
    /// for an action leaf.
    pub(crate) folded: bool,
    /// `true` for a `Node::Draft` leaf - an unassigned slot, styled
    /// apart from a real action.
    pub(crate) draft: bool,
    /// Where this node's own `delay` begins, if it has one - the
    /// stretch from here to `x` is time reserved but not yet running.
    /// `None` when there's no delay to show.
    pub(crate) gap_x: Option<f32>,
    /// This node's position in `animation`'s tree: child index at
    /// each depth, root first. What [`crate::SelectedAction`] compares
    /// against, so a click can name exactly which node it landed on.
    pub(crate) path: Vec<usize>,
}

/// Every box in `animation`'s tree, depth-first. `animation` itself
/// gets a box too, at depth `0`, as the timeline's outer frame.
///
/// `folded` names every block whose children are collapsed away - its
/// duration is unaffected, only its height and its children's boxes.
pub(crate) fn layout(
    animation: &Block<Backend>,
    folded: &BTreeSet<Vec<usize>>,
) -> Vec<Placed> {
    let root = measure_block(
        animation,
        Duration::ZERO,
        folded,
        &mut Vec::new(),
    );
    let mut out = Vec::new();
    flatten(&root, 0.0, 0, &mut Vec::new(), &mut out);
    out
}

/// A subtree's resolved extent and, if it's a block, its own laid-out
/// children (each tagged with its lane's vertical offset, relative to
/// this block's content area).
struct Measured {
    start: Duration,
    /// This node's own duration - `node_duration`/`block_duration` -
    /// what its box is drawn to.
    end: Duration,
    /// This node's own `delay` - zero for the tree's root, which has
    /// no `Node` of its own to carry one.
    gap: Duration,
    height: f32,
    kind: MeasuredKind,
}

enum MeasuredKind {
    Action {
        name: Option<String>,
    },
    Draft {
        name: Option<String>,
    },
    Block {
        label: String,
        folded: bool,
        children: Vec<(f32, Measured)>,
    },
}

/// A block's own header text: its name if set, its combinator
/// otherwise.
fn block_label(block: &Block<Backend>) -> String {
    block
        .name
        .clone()
        .unwrap_or_else(|| combinator_label(&block.combinator))
}

fn combinator_label(combinator: &Combinator) -> String {
    match combinator {
        Combinator::Chain => "Chain".into(),
        Combinator::All => "All".into(),
        Combinator::Flow(delay) => {
            format!("Flow {:.2}s", delay.as_secs_f32())
        }
    }
}

/// A node's own duration, including its `delay`: how much it advances
/// its parent block's chain/flow position. Mirrors the timing math
/// `motiongfx::track`'s `chain`/`flow` apply at compile time. Not
/// affected by folding.
fn node_duration(node: &Node<Backend>) -> Duration {
    let (delay, inner) = match node {
        Node::Action { delay, action } => (delay, action.duration),
        Node::Block { delay, block } => {
            (delay, block_duration(block))
        }
        Node::Draft {
            delay, duration, ..
        } => (delay, *duration),
    };
    inner.saturating_add(delay.unwrap_or_default())
}

/// A block's total duration under its combinator - see
/// `motiongfx::track`'s `chain`/`all`/`any`/`flow` for the runtime
/// equivalent this mirrors.
fn block_duration(block: &Block<Backend>) -> Duration {
    match block.combinator {
        Combinator::Chain => block
            .children
            .iter()
            .map(node_duration)
            .fold(Duration::ZERO, |acc, d| acc.saturating_add(d)),
        Combinator::All => {
            { block.children.iter().map(node_duration).max() }
                .unwrap_or_default()
        }
        Combinator::Flow(delay) => block
            .children
            .iter()
            .enumerate()
            .map(|(i, child)| {
                delay
                    .saturating_mul(i as u32)
                    .saturating_add(node_duration(child))
            })
            .max()
            .unwrap_or_default(),
    }
}

fn measure_node(
    node: &Node<Backend>,
    start: Duration,
    folded: &BTreeSet<Vec<usize>>,
    path: &mut Vec<usize>,
) -> Measured {
    let delay = match node {
        Node::Action { delay, .. }
        | Node::Block { delay, .. }
        | Node::Draft { delay, .. } => delay.unwrap_or_default(),
    };
    let start = start.saturating_add(delay);

    match node {
        Node::Action { action, .. } => Measured {
            start,
            end: start.saturating_add(action.duration),
            gap: delay,
            height: ROW_HEIGHT,
            kind: MeasuredKind::Action {
                name: action.name.clone(),
            },
        },
        Node::Draft { duration, name, .. } => Measured {
            start,
            end: start.saturating_add(*duration),
            gap: delay,
            height: ROW_HEIGHT,
            kind: MeasuredKind::Draft { name: name.clone() },
        },
        Node::Block { block, .. } => Measured {
            gap: delay,
            ..measure_block(block, start, folded, path)
        },
    }
}

fn measure_block(
    block: &Block<Backend>,
    start: Duration,
    folded: &BTreeSet<Vec<usize>>,
    path: &mut Vec<usize>,
) -> Measured {
    let is_folded = folded.contains(path.as_slice());
    let (children, content_height) = if is_folded {
        (Vec::new(), 0.0)
    } else {
        measure_children(
            &block.children,
            &block.combinator,
            start,
            folded,
            path,
        )
    };
    Measured {
        start,
        end: start.saturating_add(block_duration(block)),
        // Overwritten by `measure_node` for anything but the tree's
        // root, which calls this directly with no `Node::Block`
        // delay to carry.
        gap: Duration::ZERO,
        height: HEADER_HEIGHT + content_height,
        kind: MeasuredKind::Block {
            label: block_label(block),
            folded: is_folded,
            children,
        },
    }
}

/// Measures every child, then lays them into lanes (rows).
///
/// `All`/`Flow` give each child its own dedicated lane, always -
/// packing them would occasionally let two children share a lane (a
/// `Flow`'s first and fifth child, say, once the first has finished),
/// which reads as one fused bar instead of two separate actions.
///
/// A `Chain`'s children share one row, since they never overlap by
/// construction: each one starts only once its predecessor's duration
/// (plus its own `delay`) has elapsed.
fn measure_children(
    children: &[Node<Backend>],
    combinator: &Combinator,
    block_start: Duration,
    folded: &BTreeSet<Vec<usize>>,
    path: &mut Vec<usize>,
) -> (Vec<(f32, Measured)>, f32) {
    if children.is_empty() {
        return (Vec::new(), 0.0);
    }

    let starts: Vec<Duration> = match *combinator {
        Combinator::Chain => {
            let mut t = block_start;
            children
                .iter()
                .map(|child| {
                    let start = t;
                    t = t.saturating_add(node_duration(child));
                    start
                })
                .collect()
        }
        Combinator::All => {
            children.iter().map(|_| block_start).collect()
        }
        Combinator::Flow(delay) => (0..children.len())
            .map(|i| {
                block_start
                    .saturating_add(delay.saturating_mul(i as u32))
            })
            .collect(),
    };

    let measured: Vec<Measured> = children
        .iter()
        .zip(starts)
        .enumerate()
        .map(|(i, (child, start))| {
            path.push(i);
            let measured = measure_node(child, start, folded, path);
            path.pop();
            measured
        })
        .collect();

    let ys: Vec<f32> = match combinator {
        // Children never overlap in time by construction, so they all
        // share one row.
        Combinator::Chain => vec![0.0; measured.len()],
        // `All`/`Flow` children can genuinely overlap in time, so each
        // always gets its own dedicated row.
        Combinator::All | Combinator::Flow(_) => {
            let mut y = 0.0;
            measured
                .iter()
                .map(|m| {
                    let this = y;
                    y += m.height + LANE_GAP;
                    this
                })
                .collect()
        }
    };

    let content_height = measured
        .iter()
        .zip(&ys)
        .map(|(m, &y)| y + m.height)
        .fold(0.0f32, f32::max);

    let placed = ys.into_iter().zip(measured).collect();
    (placed, content_height)
}

fn flatten(
    measured: &Measured,
    y: f32,
    depth: usize,
    path: &mut Vec<usize>,
    out: &mut Vec<Placed>,
) {
    let x = crate::px_for(measured.start);
    let w =
        crate::px_for(measured.end.saturating_sub(measured.start))
            .max(MIN_WIDTH);
    let gap_x = (measured.gap > Duration::ZERO).then(|| {
        crate::px_for(measured.start.saturating_sub(measured.gap))
    });

    match &measured.kind {
        MeasuredKind::Action { name } => out.push(Placed {
            x,
            y,
            w,
            h: measured.height,
            depth,
            label: None,
            name: name.clone(),
            folded: false,
            draft: false,
            gap_x,
            path: path.clone(),
        }),
        MeasuredKind::Draft { name } => out.push(Placed {
            x,
            y,
            w,
            h: measured.height,
            depth,
            label: None,
            name: name.clone(),
            folded: false,
            draft: true,
            gap_x,
            path: path.clone(),
        }),
        MeasuredKind::Block {
            label,
            folded,
            children,
        } => {
            out.push(Placed {
                x,
                y,
                w,
                h: measured.height,
                depth,
                label: Some(label.clone()),
                name: None,
                folded: *folded,
                draft: false,
                gap_x,
                path: path.clone(),
            });
            let content_top = y + HEADER_HEIGHT;
            for (i, (lane_y, child)) in children.iter().enumerate() {
                path.push(i);
                flatten(
                    child,
                    content_top + lane_y,
                    depth + 1,
                    path,
                    out,
                );
                path.pop();
            }
        }
    }
}
