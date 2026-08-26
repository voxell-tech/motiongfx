//! [`Scene::compile`] turns a scene into a runtime [`Timeline`].
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
use crate::registry::SceneRegistry;
use crate::scene::Scene;

impl<B: SceneBackend> Scene<B> {
    /// Compiles this scene into a [`Timeline`], resolving everything it
    /// references through `scene_registry`.
    pub fn compile(
        &self,
        scene_registry: &SceneRegistry<B>,
        runtime_registry: &mut Registry,
    ) -> Result<Timeline<B::World>, CompileError<B>> {
        scene_registry.install_accessors(runtime_registry);
        let mut builder =
            runtime_registry.create_builder::<B::World>();

        let root_fragment = walk_block(
            &self.animation,
            scene_registry,
            &self.values,
            &mut builder,
        )?;

        let track = root_fragment.compile();

        Ok(builder.compile(track))
    }

    /// Writes the [`Stage`](crate::scene::Stage)'s initial values into
    /// `world`. Run after the subjects are materialized, before
    /// `bake_actions`.
    pub fn stage(
        &self,
        scene_registry: &SceneRegistry<B>,
        world: &mut B::World,
    ) -> Result<(), CompileError<B>> {
        for subject in &self.stage.subjects {
            for seed in &subject.fields {
                scene_registry.seed_field(
                    subject.id,
                    seed,
                    &self.values,
                    world,
                )?;
            }
        }

        Ok(())
    }
}

/// Compiles a [`Node`] into a [`TrackFragment`].
fn walk_node<B: SceneBackend>(
    node: &Node<B>,
    registry: &SceneRegistry<B>,
    values: &B::ValuePool,
    builder: &mut TimelineBuilder<'_, B::World>,
) -> Result<TrackFragment, CompileError<B>> {
    let (offset, fragment) = match node {
        Node::Block { delay, block } => {
            (delay, walk_block(block, registry, values, builder)?)
        }
        Node::Action { delay, action } => (
            delay,
            resolve_action(action, registry, values, builder)?,
        ),
        Node::Draft {
            delay, duration, ..
        } => (delay, TrackFragment::silent(*duration)),
    };

    Ok(match offset {
        Some(offset) => delay(*offset, fragment),
        None => fragment,
    })
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
