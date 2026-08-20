//! The backend contract.

use alloc::vec::Vec;
use core::hash::Hash;

pub trait Host: Sized + Send + Sync + 'static {
    /// Opaque handle to a node.
    type Node: Copy + Eq + Hash + Send + Sync + 'static;

    /// The backend's world.
    type World: 'static;

    /// Read-only context passed to every
    /// [`Style::apply`](crate::style::Style::apply), usually a theme.
    /// Owned by [`Fynix`](crate::Fynix), not `World`.
    type Theme: 'static;

    /// Seconds since the last flush. What a transition advances by.
    fn delta(world: &Self::World) -> f32;

    /// Create an empty node under `parent`, wired for
    /// [`Host::children`]/[`Host::despawn`] to find later.
    fn spawn(
        world: &mut Self::World,
        parent: Self::Node,
    ) -> Self::Node;

    /// Whether `node` is still alive.
    fn exists(world: &Self::World, node: Self::Node) -> bool;

    /// Direct children of `node`, in order.
    fn children(
        world: &Self::World,
        node: Self::Node,
    ) -> Vec<Self::Node>;

    /// Destroy `node` and everything beneath it.
    fn despawn(world: &mut Self::World, node: Self::Node);
}
