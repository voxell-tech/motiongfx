//! The ECS as a [`Host`].

use core::marker::PhantomData;

use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::*;
use bevy_time::Time;
use bevy_ui::Node;
use fynix_mock::host::Host;

/// Generic over `Theme`. This crate never names the app's concrete
/// theme, only that it's a [`Resource`].
pub struct BevyHost<Theme>(PhantomData<fn() -> Theme>);

impl<Theme: Resource + Clone + Default> Host for BevyHost<Theme> {
    type Node = Entity;
    type World = World;
    type Theme = Theme;

    fn delta(world: &World) -> f32 {
        world
            .get_resource::<Time>()
            .map(|time| time.delta_secs())
            .unwrap_or_default()
    }

    fn spawn(world: &mut World, parent: Entity) -> Entity {
        // A layout `Node` up front. Without one, bevy warns B0004 and
        // skips this entity's children in layout. An element that
        // brings its own overwrites this.
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
