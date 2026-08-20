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

use bevy_motiongfx::scene::backend::Backend;
use motiongfx_scene::block::{Block, Combinator, Node};

/// Height of an action leaf's bar, and of a block's header strip.
const ROW_HEIGHT: f32 = 20.0;
const HEADER_HEIGHT: f32 = 20.0;
/// Vertical gap between lanes that would otherwise overlap in time.
const LANE_GAP: f32 = 4.0;
const MIN_WIDTH: f32 = 2.0;

/// One box to draw: a block header (its combinator, as a label) or an
/// action leaf.
#[derive(Clone, PartialEq)]
pub(crate) struct Placed {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) depth: usize,
    /// `Some` for a block header box; `None` for an action leaf.
    pub(crate) label: Option<String>,
    /// This node's position in `animation`'s tree: child index at
    /// each depth, root first. What [`crate::SelectedAction`] compares
    /// against, so a click can name exactly which node it landed on.
    pub(crate) path: Vec<usize>,
}

/// Every box in `animation`'s tree, depth-first. `animation` itself
/// gets a box too, at depth `0`, as the timeline's outer frame.
pub(crate) fn layout(animation: &Block<Backend>) -> Vec<Placed> {
    let root = measure_block(animation, Duration::ZERO);
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
    /// what its *own* box is drawn to. A block's box is only ever
    /// this wide, `Any` included: it marks the group's boundary, not
    /// how long its content happens to keep drawing past it.
    end: Duration,
    /// The rightmost point any of this node's own content actually
    /// draws to - used only to detect whether a following `Chain`
    /// sibling needs to drop to a new row, never to size a box. Equal
    /// to `end` everywhere except an `Any`, whose losing branch keeps
    /// animating (and drawing) past it, and any block containing one,
    /// recursively.
    visual_end: Duration,
    height: f32,
    kind: MeasuredKind,
}

enum MeasuredKind {
    Action,
    Block {
        label: String,
        children: Vec<(f32, Measured)>,
    },
}

fn combinator_label(combinator: &Combinator) -> String {
    match combinator {
        Combinator::Chain => "Chain".into(),
        Combinator::All => "All".into(),
        Combinator::Any => "Any".into(),
        Combinator::Flow(delay) => {
            format!("Flow {:.2}s", delay.as_secs_f32())
        }
    }
}

/// A node's own duration, including its `delay`: how much it advances
/// its parent block's chain/flow position. Mirrors the timing math
/// `motiongfx::track`'s `chain`/`flow` apply at compile time.
fn node_duration(node: &Node<Backend>) -> Duration {
    let (delay, inner) = match node {
        Node::Action { delay, action } => (delay, action.duration),
        Node::Block { delay, block } => {
            (delay, block_duration(block))
        }
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
        Combinator::Any => {
            { block.children.iter().map(node_duration).min() }
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

fn measure_node(node: &Node<Backend>, start: Duration) -> Measured {
    let delay = match node {
        Node::Action { delay, .. } | Node::Block { delay, .. } => {
            delay.unwrap_or_default()
        }
    };
    let start = start.saturating_add(delay);

    match node {
        Node::Action { action, .. } => {
            let end = start.saturating_add(action.duration);
            Measured {
                start,
                end,
                visual_end: end,
                height: ROW_HEIGHT,
                kind: MeasuredKind::Action,
            }
        }
        Node::Block { block, .. } => measure_block(block, start),
    }
}

fn measure_block(
    block: &Block<Backend>,
    start: Duration,
) -> Measured {
    let (children, content_height) =
        measure_children(&block.children, &block.combinator, start);
    let visual_end = children
        .iter()
        .map(|(_, m)| m.visual_end)
        .max()
        .unwrap_or(start);
    Measured {
        start,
        end: start.saturating_add(block_duration(block)),
        visual_end,
        height: HEADER_HEIGHT + content_height,
        kind: MeasuredKind::Block {
            label: combinator_label(&block.combinator),
            children,
        },
    }
}

/// Measures every child, then lays them into lanes (rows).
///
/// `All`/`Any`/`Flow` give each child its own dedicated lane, always -
/// packing them would occasionally let two children share a lane (a
/// `Flow`'s first and fifth child, say, once the first has finished),
/// which reads as one fused bar instead of two separate actions.
///
/// A `Chain`'s children share one row by default, since they never
/// overlap by construction - except an `Any` child's visual extent
/// can bleed past its official contribution to the chain (its losing
/// branch keeps animating), reaching into where the next sibling
/// starts. When it does, only *that* sibling drops below - directly
/// under the one predecessor it actually overlaps, not under whatever
/// else happens to share the row - since a chain's children are
/// already time-ordered and only ever need to check the one right
/// before them.
fn measure_children(
    children: &[Node<Backend>],
    combinator: &Combinator,
    block_start: Duration,
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
        Combinator::All | Combinator::Any => {
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
        .map(|(child, start)| measure_node(child, start))
        .collect();

    let ys: Vec<f32> = match combinator {
        Combinator::Chain => {
            let mut ys = Vec::with_capacity(measured.len());
            let mut prev: Option<(Duration, f32, f32)> = None;
            for m in &measured {
                let y = match prev {
                    Some((end, y, height)) if m.start < end => {
                        y + height + LANE_GAP
                    }
                    _ => 0.0,
                };
                ys.push(y);
                prev = Some((m.visual_end, y, m.height));
            }
            ys
        }
        // Every other combinator's children can genuinely overlap in
        // time, so each always gets its own dedicated row.
        Combinator::All | Combinator::Any | Combinator::Flow(_) => {
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

    match &measured.kind {
        MeasuredKind::Action => out.push(Placed {
            x,
            y,
            w,
            h: measured.height,
            depth,
            label: None,
            path: path.clone(),
        }),
        MeasuredKind::Block { label, children } => {
            out.push(Placed {
                x,
                y,
                w,
                h: measured.height,
                depth,
                label: Some(label.clone()),
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
