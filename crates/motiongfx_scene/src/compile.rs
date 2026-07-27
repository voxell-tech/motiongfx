//! Compiling a [`Scene`] into a runtime [`Timeline`].
//!
//! 1. Walk the block tree, resolving each [`ActionCmd`] through
//!    [`SceneRegistry`] into a [`TrackFragment`].
//! 2. Fold each [`Block`]'s children by its [`Combinator`].
//! 3. Compile the root fragment into a [`Timeline`].

use alloc::vec::Vec;

use motiongfx::prelude::*;
use motiongfx::registry::Registry;
use motiongfx::subject::SubjectId;
use motiongfx::track::delay;

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
pub fn compile<Id, V, W>(
    scene: &Scene<Id, V>,
    scene_registry: &SceneRegistry<Id, V, W>,
    runtime_registry: &mut Registry,
) -> Result<Timeline<W>, CompileError<Id>>
where
    Id: SubjectId,
    V: 'static,
    W: 'static,
{
    scene_registry.install_accessors(runtime_registry);
    let mut builder = runtime_registry.create_builder::<W>();

    let root_fragment =
        walk_block(&scene.animation, scene_registry, &mut builder)?;

    let track = root_fragment.compile();
    builder.add_tracks([track]);

    builder.try_compile().ok_or_else(|| {
        CompileError::UnknownField(FieldRef {
            type_name: "".into(),
            path: "".into(),
        })
    })
}

/// Compiles a [`Node`] into a [`TrackFragment`].
fn walk_node<Id, V, W>(
    node: &Node<Id, V>,
    registry: &SceneRegistry<Id, V, W>,
    builder: &mut TimelineBuilder<'_, W>,
) -> Result<TrackFragment, CompileError<Id>>
where
    Id: SubjectId,
    V: 'static,
    W: 'static,
{
    match node {
        Node::Block(block) => walk_block(block, registry, builder),
        Node::Action(cmd) => resolve_action(cmd, registry, builder),
        Node::Delayed { offset, node } => {
            let fragment = walk_node(node, registry, builder)?;
            Ok(delay(*offset, fragment))
        }
    }
}

/// Compiles a [`Block`] into a [`TrackFragment`].
fn walk_block<Id, V, W>(
    block: &Block<Id, V>,
    registry: &SceneRegistry<Id, V, W>,
    builder: &mut TimelineBuilder<'_, W>,
) -> Result<TrackFragment, CompileError<Id>>
where
    Id: SubjectId,
    V: 'static,
    W: 'static,
{
    let mut fragments = Vec::with_capacity(block.children.len());

    for child in &block.children {
        let fragment = walk_node(child, registry, builder)?;
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

fn resolve_action<Id, V, W>(
    cmd: &ActionCmd<Id, V>,
    registry: &SceneRegistry<Id, V, W>,
    builder: &mut TimelineBuilder<'_, W>,
) -> Result<TrackFragment, CompileError<Id>>
where
    Id: SubjectId,
    V: 'static,
    W: 'static,
{
    registry.resolve_op(cmd, builder)
}
