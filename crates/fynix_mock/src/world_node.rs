//! The `(world, node)` pair as one type.

use crate::host::Host;

/// A backend world and a node in it, the world borrowed shared.
pub struct WorldNodeRef<'w, H: Host> {
    pub world: &'w H::World,
    pub node: H::Node,
}

impl<'w, H: Host> WorldNodeRef<'w, H> {
    pub fn new(world: &'w H::World, node: H::Node) -> Self {
        Self { world, node }
    }
}

/// [`WorldNodeRef`] with the world borrowed exclusively.
pub struct WorldNodeMut<'w, H: Host> {
    pub world: &'w mut H::World,
    pub node: H::Node,
}

impl<'w, H: Host> WorldNodeMut<'w, H> {
    pub fn new(world: &'w mut H::World, node: H::Node) -> Self {
        Self { world, node }
    }

    /// A shared view of the same pair.
    pub fn as_ref(&self) -> WorldNodeRef<'_, H> {
        WorldNodeRef {
            world: self.world,
            node: self.node,
        }
    }

    /// A shorter-lived exclusive borrow of the same pair, for handing
    /// on without giving up ownership.
    pub fn reborrow(&mut self) -> WorldNodeMut<'_, H> {
        WorldNodeMut {
            world: self.world,
            node: self.node,
        }
    }
}
