//! Compiling a [`Scene`] into a runtime [`Timeline`].
//!
//! 1. Walk the block tree, resolving each [`ActionCmd`] through
//!    [`SceneRegistry`] into a [`TrackFragment`].
//! 2. Fold each [`Block`]'s children by its [`Combinator`].
//! 3. Compile the root fragment into a [`Timeline`].

use alloc::vec::Vec;

use motiongfx::prelude::*;
use motiongfx::registry::Registry;
use motiongfx::track::delay;

use crate::backend::SceneBackend;
use crate::block::{ActionCmd, Block, Combinator, Node};
use crate::error::CompileError;
use crate::refs::FieldRef;
use crate::registry::SceneRegistry;
use crate::scene::Scene;

/// Compiles a [`Scene`] into a [`Timeline`].
///
/// # Errors
///
/// Returns [`CompileError`] if any subject, field, op, ease, or interp
/// referenced in the scene cannot be resolved through `scene_registry`.
pub fn compile<B: SceneBackend>(
    scene: &Scene<B>,
    scene_registry: &SceneRegistry<B>,
    runtime_registry: &mut Registry,
) -> Result<Timeline<B::World>, CompileError<B>> {
    scene_registry.install_accessors(runtime_registry);
    let mut builder = runtime_registry.create_builder::<B::World>();

    let root_fragment = walk_block(
        &scene.animation,
        scene_registry,
        &scene.values,
        &mut builder,
    )?;

    let track = root_fragment.compile();
    builder.add_tracks([track]);

    builder.try_compile().ok_or_else(|| {
        CompileError::UnknownField(FieldRef::new("", ""))
    })
}

/// Compiles a [`Node`] into a [`TrackFragment`].
fn walk_node<B: SceneBackend>(
    node: &Node<B>,
    registry: &SceneRegistry<B>,
    values: &B::ValuePool,
    builder: &mut TimelineBuilder<'_, B::World>,
) -> Result<TrackFragment, CompileError<B>> {
    match node {
        Node::Block(block) => {
            walk_block(block, registry, values, builder)
        }
        Node::Action(cmd) => {
            resolve_action(cmd, registry, values, builder)
        }
        Node::Delayed { offset, node } => {
            let fragment =
                walk_node(node, registry, values, builder)?;
            Ok(delay(*offset, fragment))
        }
    }
}

/// Compiles a [`Block`] into a [`TrackFragment`].
fn walk_block<B: SceneBackend>(
    block: &Block<B>,
    registry: &SceneRegistry<B>,
    values: &B::ValuePool,
    builder: &mut TimelineBuilder<'_, B::World>,
) -> Result<TrackFragment, CompileError<B>> {
    let mut fragments = Vec::with_capacity(block.children.len());

    for child in &block.children {
        let fragment = walk_node(child, registry, values, builder)?;
        fragments.push(fragment);
    }

    let result = match block.combinator {
        Combinator::Chain => fragments.ord_chain(),
        Combinator::All => fragments.ord_all(),
        Combinator::Any => fragments.ord_any(),
        Combinator::Flow(delay) => fragments.ord_flow(delay),
    };

    Ok(result)
}

fn resolve_action<B: SceneBackend>(
    cmd: &ActionCmd<B>,
    registry: &SceneRegistry<B>,
    values: &B::ValuePool,
    builder: &mut TimelineBuilder<'_, B::World>,
) -> Result<TrackFragment, CompileError<B>> {
    registry.resolve_op(cmd, values, builder)
}
