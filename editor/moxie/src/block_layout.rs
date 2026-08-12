//! Lays a scene's [`Block`] tree out as nested boxes: a block is a
//! bordered container spanning its time range, holding its children -
//! actions as filled bars, nested blocks as containers of their own -
//! rather than one flat row per node.
//!
//! Horizontal position always comes straight from a node's resolved
//! start time ([`crate::px_for`]); nesting only ever affects the
//! *vertical* axis, so a block's box literally encloses its children's
//! boxes instead of merely implying the relationship through
//! indentation.

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
    /// `true` for the part of an `Any`'s losing branch that plays on
    /// past the group's official end - see [`layout`].
    pub(crate) dotted: bool,
}

/// Every box in `animation`'s tree, depth-first - `animation` itself
/// gets a box too (depth `0`), playing the role of the whole
/// timeline's outer frame.
///
/// An `Any` ends the instant its fastest child does, but a slower
/// sibling keeps animating past that - so its box gets split at that
/// point: solid up to there, `dotted` beyond. Every `dotted` piece is
/// appended last, after every normal box, so it always paints on top
/// instead of ending up hidden under whatever the timeline places
/// next to the `Any` block.
pub(crate) fn layout(animation: &Block<Backend>) -> Vec<Placed> {
    let root = measure_block(animation, Duration::ZERO);
    let mut out = Vec::new();
    let mut overlay = Vec::new();
    flatten(&root, 0.0, 0, &mut out, &mut overlay);
    out.extend(overlay);
    out
}

/// A subtree's resolved extent and, if it's a block, its own laid-out
/// children (each tagged with its lane's vertical offset, relative to
/// this block's content area).
struct Measured {
    start: Duration,
    end: Duration,
    height: f32,
    kind: MeasuredKind,
}

enum MeasuredKind {
    Action,
    Block {
        label: String,
        /// Whether this block is an `Any` - the only combinator whose
        /// children can individually outlast the block's own official
        /// end, which is what [`flatten`] checks before splitting one.
        is_any: bool,
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
        Node::Action { action, .. } => Measured {
            start,
            end: start.saturating_add(action.duration),
            height: ROW_HEIGHT,
            kind: MeasuredKind::Action,
        },
        Node::Block { block, .. } => measure_block(block, start),
    }
}

fn measure_block(
    block: &Block<Backend>,
    start: Duration,
) -> Measured {
    let (children, content_height) =
        measure_children(&block.children, &block.combinator, start);
    Measured {
        start,
        end: start.saturating_add(block_duration(block)),
        height: HEADER_HEIGHT + content_height,
        kind: MeasuredKind::Block {
            label: combinator_label(&block.combinator),
            is_any: matches!(block.combinator, Combinator::Any),
            children,
        },
    }
}

/// Measures every child, then lays them into lanes (rows): a `Chain`'s
/// children never overlap in time by construction, so they all share
/// one lane, side by side - every other combinator gives each child
/// its own dedicated lane, always, rather than repacking lanes as
/// earlier children free up. Packing would occasionally let two
/// children share a lane (e.g. a `Flow`'s first and fifth child, once
/// the first has finished) - correct, but it reads as one fused bar
/// instead of two separate actions, which is worse than the extra
/// row it would have cost to keep them apart.
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

    let lane_of: Vec<usize> = match combinator {
        Combinator::Chain => vec![0; measured.len()],
        Combinator::All | Combinator::Any | Combinator::Flow(_) => {
            (0..measured.len()).collect()
        }
    };
    let lane_count = match combinator {
        Combinator::Chain => 1,
        Combinator::All | Combinator::Any | Combinator::Flow(_) => {
            measured.len()
        }
    };

    let mut lane_height = vec![0f32; lane_count];
    for (i, m) in measured.iter().enumerate() {
        lane_height[lane_of[i]] =
            lane_height[lane_of[i]].max(m.height);
    }
    let mut lane_y = vec![0f32; lane_height.len()];
    let mut y = 0.0;
    for (lane, h) in lane_height.iter().enumerate() {
        lane_y[lane] = y;
        y += h + LANE_GAP;
    }
    let content_height = (y - LANE_GAP).max(0.0);

    let placed = measured
        .into_iter()
        .enumerate()
        .map(|(i, m)| (lane_y[lane_of[i]], m))
        .collect();
    (placed, content_height)
}

fn flatten(
    measured: &Measured,
    y: f32,
    depth: usize,
    out: &mut Vec<Placed>,
    overlay: &mut Vec<Placed>,
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
            dotted: false,
        }),
        MeasuredKind::Block {
            label,
            is_any,
            children,
        } => {
            out.push(Placed {
                x,
                y,
                w,
                h: measured.height,
                depth,
                label: Some(label.clone()),
                dotted: false,
            });
            let content_top = y + HEADER_HEIGHT;
            for (lane_y, child) in children {
                let child_y = content_top + lane_y;
                // An `Any` can end before a slower child does; split
                // that child's own box right there instead of hiding
                // (or letting a later sibling collide with) the part
                // it keeps animating through.
                if *is_any && child.end > measured.end {
                    split_at(
                        child,
                        child_y,
                        depth + 1,
                        measured.end,
                        out,
                        overlay,
                    );
                } else {
                    flatten(child, child_y, depth + 1, out, overlay);
                }
            }
        }
    }
}

/// Renders `child`'s own box in two pieces at `bound` (its parent
/// `Any`'s official end): solid up to there, appended to `out` like
/// any other box, then `dotted` from `bound` to `child`'s own true
/// end, appended to `overlay` instead so [`layout`] can paint it last.
/// If `child` is itself a block, its descendants still render
/// normally beneath it - only its own outer box gets split.
fn split_at(
    child: &Measured,
    y: f32,
    depth: usize,
    bound: Duration,
    out: &mut Vec<Placed>,
    overlay: &mut Vec<Placed>,
) {
    let label = match &child.kind {
        MeasuredKind::Action => None,
        MeasuredKind::Block { label, .. } => Some(label.clone()),
    };

    let solid_x = crate::px_for(child.start);
    let solid_w = crate::px_for(bound.saturating_sub(child.start))
        .max(MIN_WIDTH);
    out.push(Placed {
        x: solid_x,
        y,
        w: solid_w,
        h: child.height,
        depth,
        label: label.clone(),
        dotted: false,
    });

    let dotted_x = crate::px_for(bound);
    let dotted_w =
        crate::px_for(child.end.saturating_sub(bound)).max(MIN_WIDTH);
    overlay.push(Placed {
        x: dotted_x,
        y,
        w: dotted_w,
        h: child.height,
        depth,
        label,
        dotted: true,
    });

    if let MeasuredKind::Block { children, .. } = &child.kind {
        let content_top = y + HEADER_HEIGHT;
        for (lane_y, grandchild) in children {
            flatten(
                grandchild,
                content_top + lane_y,
                depth + 1,
                out,
                overlay,
            );
        }
    }
}
