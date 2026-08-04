//! The backend contract.
//!
//! Everything the kernel needs from a UI backend, and nothing more:
//! no layout, no painting, no identity of its own. A host supplies an
//! opaque node handle and four operations over it.

use alloc::vec::Vec;
use core::hash::Hash;

pub trait Host: Send + Sync + 'static {
    /// Opaque handle to a node. The kernel never inspects it.
    type Node: Copy + Eq + Hash + Send + Sync + 'static;

    /// State the builders read and the applies write. One type, not
    /// two: the kernel only ever holds one of `&` or `&mut` at a time.
    type World: 'static;

    /// Create an empty node under `parent`.
    ///
    /// The kernel spawns rather than taking a caller-made handle
    /// because cleanup depends on it: [`Host::children`] is how a
    /// rebuild finds what to despawn and whose bindings to drop. A
    /// node that never got wired to its parent would leak on every
    /// rebuild.
    fn spawn(
        world: &mut Self::World,
        parent: Self::Node,
    ) -> Self::Node;

    /// Whether `node` is still alive. The kernel outlives the nodes it
    /// watches, so it has to be able to ask.
    fn exists(world: &Self::World, node: Self::Node) -> bool;

    /// Direct children of `node`, in order.
    fn children(
        world: &Self::World,
        node: Self::Node,
    ) -> Vec<Self::Node>;

    /// Destroy `node` and everything beneath it.
    fn despawn(world: &mut Self::World, node: Self::Node);
}
