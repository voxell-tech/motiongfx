//! Where the `#[elem(child)]` children went.
//!
//! The store remembers which node each child got, keyed by the
//! parent's node plus the field. A patch can walk down to any depth
//! without threading state from the root.

use hashbrown::HashMap;

use crate::host::Host;
use crate::lenz::{Cursor, FieldId, FieldPath, Identity};

/// The node each `#[elem(child)]` field built, per parent.
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
    /// Catches whatever the app despawned on its own.
    pub fn prune(&mut self, world: &H::World) {
        self.children.retain(|(parent, _), child| {
            H::exists(world, *parent) && H::exists(world, *child)
        });
    }

    /// The node a `#[elem(child)]` field built, however many hops
    /// the path takes to reach it.
    pub fn child<S, P>(
        &self,
        node: H::Node,
        field: impl FnOnce(Cursor<Identity<S>>) -> Cursor<P>,
    ) -> Option<H::Node>
    where
        P: FieldPath<Source = S>,
    {
        field(Cursor::new())
            .hops()
            .into_iter()
            .try_fold(node, |node, hop| self.get(node, hop))
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}
