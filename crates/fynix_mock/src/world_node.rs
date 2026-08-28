//! The `(world, node)` pair as one value.
//!
//! Bindings, watchers, and [`Lane::advance`](crate::lanes::Lane) all
//! work on one node inside the backend's world. Passing the world and
//! the node as two separate params repeats the pairing everywhere and
//! keeps a backend from offering node-level shorthands on it. These
//! two structs carry the pair as one, so a closure or a helper takes a
//! single argument, and a backend can hang an extension trait off the
//! mutable one.

use crate::host::Host;

/// A backend world and a node in it, the world borrowed shared.
///
/// What a [`ChangedFn`](crate::records::ChangedFn) and a binding's
/// value reader are handed.
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
///
/// What [`Lane::advance`](crate::lanes::Lane) works through. A backend
/// implements its own node-level extension trait for this one (see
/// `bevy_fynix`'s `EntityExt`), so a function whose whole job is to
/// mutate its node can take it and drop the manual lookup.
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
        WorldNodeRef { world: self.world, node: self.node }
    }

    /// A shorter-lived exclusive borrow of the same pair, for handing
    /// on without giving up ownership.
    pub fn reborrow(&mut self) -> WorldNodeMut<'_, H> {
        WorldNodeMut { world: self.world, node: self.node }
    }
}
