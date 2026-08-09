//! The ECS as a [`Host`].

use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::*;
use bevy_ui::Node;
use fynix_mock::host::Host;

pub struct BevyHost;

impl Host for BevyHost {
    type Node = Entity;
    type World = World;

    fn spawn(world: &mut World, parent: Entity) -> Entity {
        // A layout [`Node`] up front, not [`Host::Node`], which is the
        // `Entity` itself. An entity in the UI tree without one is
        // always a mistake: bevy warns B0004 and skips its children's
        // layout. An element that brings its own overwrites this.
        world.spawn((Node::default(), ChildOf(parent))).id()
    }

    fn exists(world: &World, node: Entity) -> bool {
        world.entities().contains(node)
    }

    fn children(world: &World, node: Entity) -> Vec<Entity> {
        world
            .get::<Children>(node)
            .map(|children| children.to_vec())
            .unwrap_or_default()
    }

    fn despawn(world: &mut World, node: Entity) {
        world.despawn(node);
    }
}
