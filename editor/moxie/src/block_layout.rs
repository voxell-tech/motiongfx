//! Flattens a scene's [`Block`] tree into timeline rows: one per node,
//! depth-first, each carrying its resolved start time and duration.
//!
//! Mirrors the timing math `motiongfx::track`'s `chain`/`all`/`any`/
//! `flow` apply when a [`Block`] compiles into a `Track`, but stays
//! entirely off the registry: every leaf's `duration` sits right on
//! its [`ActionCmd`](motiongfx_scene::block::ActionCmd), so a row's
//! start/duration needs nothing beyond the tree itself.

use core::time::Duration;

use bevy_motiongfx::scene::backend::Backend;
use motiongfx_scene::block::{Block, Combinator, Node};

/// One row in the timeline panel: a [`Node::Block`] header or a
/// [`Node::Action`] leaf, at its resolved start time.
#[derive(Clone, PartialEq)]
pub(crate) struct Row {
    pub(crate) depth: usize,
    pub(crate) start: Duration,
    pub(crate) duration: Duration,
    /// `Some` for a block header row (its combinator); `None` for an
    /// action leaf.
    pub(crate) combinator: Option<Combinator>,
}

/// Every row of `block`'s subtree, depth-first. `block` itself gets no
/// row of its own - its children start at depth `0`, matching the
/// timeline panel's previous one-row-per-top-level-track look.
pub(crate) fn rows(block: &Block<Backend>) -> Vec<Row> {
    let mut out = Vec::new();
    layout_block(block, 0, Duration::ZERO, &mut out);
    out
}

/// A node's own duration, including its `delay`: how much it advances
/// its parent block's chain/flow position.
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

fn layout_node(
    node: &Node<Backend>,
    depth: usize,
    start: Duration,
    out: &mut Vec<Row>,
) {
    let delay = match node {
        Node::Action { delay, .. } | Node::Block { delay, .. } => {
            delay.unwrap_or_default()
        }
    };
    let start = start.saturating_add(delay);

    match node {
        Node::Action { action, .. } => out.push(Row {
            depth,
            start,
            duration: action.duration,
            combinator: None,
        }),
        Node::Block { block, .. } => {
            out.push(Row {
                depth,
                start,
                duration: block_duration(block),
                combinator: Some(block.combinator.clone()),
            });
            layout_block(block, depth + 1, start, out);
        }
    }
}

fn layout_block(
    block: &Block<Backend>,
    depth: usize,
    start: Duration,
    out: &mut Vec<Row>,
) {
    match block.combinator {
        Combinator::Chain => {
            let mut t = start;
            for child in &block.children {
                layout_node(child, depth, t, out);
                t = t.saturating_add(node_duration(child));
            }
        }
        Combinator::All | Combinator::Any => {
            for child in &block.children {
                layout_node(child, depth, start, out);
            }
        }
        Combinator::Flow(delay) => {
            for (i, child) in block.children.iter().enumerate() {
                let child_start = start
                    .saturating_add(delay.saturating_mul(i as u32));
                layout_node(child, depth, child_start, out);
            }
        }
    }
}
