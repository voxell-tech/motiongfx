//! Where the `#[elem]` children went.
//!
//! A parent builds its children and forgets them; the store remembers
//! which node each one got, so a later patch can walk down to it. The
//! key is the parent's node plus the field, which is enough on its
//! own: nothing has to be threaded down from the root, and the same
//! lookup works at any depth.

use hashbrown::HashMap;

use crate::host::Host;
use crate::lenz::FieldId;

/// The node each `#[elem]` field built, per parent.
pub struct Store<H: Host> {
    children: HashMap<(H::Node, FieldId), H::Node>,
}

impl<H: Host> Default for Store<H> {
    fn default() -> Self {
        Self {
            children: HashMap::new(),
        }
    }
}

impl<H: Host> Store<H> {
    pub fn new() -> Self {
        Self::default()
    }

    /// The node `field` built under `parent`.
    pub fn get(
        &self,
        parent: H::Node,
        field: FieldId,
    ) -> Option<H::Node> {
        self.children.get(&(parent, field)).copied()
    }

    pub fn insert(
        &mut self,
        parent: H::Node,
        field: FieldId,
        child: H::Node,
    ) {
        self.children.insert((parent, field), child);
    }

    /// Forget `field`'s child and hand it back, for a teardown.
    pub fn take(
        &mut self,
        parent: H::Node,
        field: FieldId,
    ) -> Option<H::Node> {
        self.children.remove(&(parent, field))
    }

    /// Drop entries whose nodes the backend no longer has.
    ///
    /// A teardown through [`Element::despawn`](crate::element::Element::despawn)
    /// clears its own entries as it goes. This is the sweep for what
    /// the app despawned behind our back.
    pub fn prune(&mut self, world: &H::World) {
        self.children.retain(|(parent, _), child| {
            H::exists(world, *parent) && H::exists(world, *child)
        });
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}
