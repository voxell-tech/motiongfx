//! The builder.
//!
//! [`Ui`] holds `&mut World` and spawns as it goes, so a builder gets
//! each node's handle the moment it makes one. That is what lets a
//! binding refer to a *sibling* or *parent* by handle instead of
//! hunting for it at poll time.
//!
//! The cost is that a builder cannot hold a borrow of the world across
//! a spawn: collect what you need first, then build from it.

use alloc::boxed::Box;
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::host::Host;

/// Predicate over the world, polled once per flush.
///
/// `FnMut` because a predicate usually diffs against what it last saw,
/// and that value has to live somewhere. It must be called exactly
/// once per flush: a stateful predicate consumes its own signal.
pub type ChangedFn<H> = Box<
    dyn FnMut(&<H as Host>::World, <H as Host>::Node) -> bool
        + Send
        + Sync,
>;

/// Writes into one node. Already type-erased by the caller, so the
/// kernel never needs a typed accessor of its own.
pub type ApplyFn<H> = Box<
    dyn Fn(&mut <H as Host>::World, <H as Host>::Node) + Send + Sync,
>;

/// Rebuilds the subtree under a node.
pub type BuildFn<H> =
    Box<dyn for<'a> Fn(&mut Ui<'a, H>) + Send + Sync>;

/// A field binding: when `changed` fires, `apply` runs.
pub struct Binding<H: Host> {
    pub(crate) changed: ChangedFn<H>,
    pub(crate) apply: ApplyFn<H>,
}

/// A watcher rooted at a node.
pub struct Watcher<H: Host> {
    pub(crate) root: H::Node,
    pub(crate) changed: ChangedFn<H>,
    pub(crate) build: BuildFn<H>,
}

/// What a build registers as it runs, kept beside the world so both
/// can be borrowed at once.
pub struct Records<H: Host> {
    pub(crate) bindings: HashMap<H::Node, Vec<Binding<H>>>,
    /// Watchers declared during a build. They cannot go straight into
    /// the kernel's list, which is mid-iteration, and must not run
    /// until the next flush anyway.
    pub(crate) spawned: Vec<Watcher<H>>,
}

impl<H: Host> Default for Records<H> {
    fn default() -> Self {
        Self {
            bindings: HashMap::new(),
            spawned: Vec::new(),
        }
    }
}

/// Spawns nodes under a parent and records their reactivity.
pub struct Ui<'a, H: Host> {
    world: &'a mut H::World,
    parent: H::Node,
    records: &'a mut Records<H>,
}

impl<'a, H: Host> Ui<'a, H> {
    pub(crate) fn new(
        world: &'a mut H::World,
        parent: H::Node,
        records: &'a mut Records<H>,
    ) -> Self {
        Self {
            world,
            parent,
            records,
        }
    }

    /// The world, for reads. Collect what you need from it before
    /// spawning: the borrow cannot outlive the next builder call.
    pub fn world(&self) -> &H::World {
        self.world
    }

    /// The node these children are being built under.
    pub fn parent(&self) -> H::Node {
        self.parent
    }

    /// Spawn a node and fill it with `widget`.
    pub fn node(
        &mut self,
        widget: impl FnOnce(&mut H::World, H::Node),
    ) -> NodeMut<'_, 'a, H> {
        let node = H::spawn(self.world, self.parent);
        widget(self.world, node);
        NodeMut { ui: self, node }
    }

    /// Spawn a node with nothing in it: for grouping, or to scope a
    /// binding whose write lands somewhere other than a node.
    pub fn empty_node(&mut self) -> NodeMut<'_, 'a, H> {
        let node = H::spawn(self.world, self.parent);
        NodeMut { ui: self, node }
    }
}

/// A freshly spawned node, for chaining children and bindings.
pub struct NodeMut<'u, 'a, H: Host> {
    ui: &'u mut Ui<'a, H>,
    node: H::Node,
}

impl<H: Host> NodeMut<'_, '_, H> {
    /// This node's handle. Capture it to bind a *later* node against
    /// this one.
    pub fn id(&self) -> H::Node {
        self.node
    }

    /// Fill this node with `widget`, for nodes spawned without one.
    pub fn widget(
        self,
        widget: impl FnOnce(&mut H::World, H::Node),
    ) -> Self {
        widget(self.ui.world, self.node);
        self
    }

    /// Rebuild this node's children whenever `changed` fires. First
    /// runs on the next flush.
    ///
    /// Use this *or* [`Self::with`], not both: a fire clears whatever
    /// children the node has.
    pub fn watch(
        self,
        changed: impl FnMut(&H::World, H::Node) -> bool
        + Send
        + Sync
        + 'static,
        build: impl for<'b> Fn(&mut Ui<'b, H>) + Send + Sync + 'static,
    ) -> Self {
        self.ui.records.spawned.push(Watcher {
            root: self.node,
            changed: Box::new(changed),
            build: Box::new(build),
        });
        self
    }

    /// Build children under this node.
    pub fn with(self, f: impl FnOnce(&mut Ui<'_, H>)) -> Self {
        let mut child =
            Ui::new(self.ui.world, self.node, self.ui.records);
        f(&mut child);
        self
    }

    /// Run `apply` on this node whenever `changed` fires.
    ///
    /// Both closures are already erased. Typed conveniences belong to
    /// the host adapter, which still has the concrete types in scope.
    pub fn bind_raw(
        self,
        changed: impl FnMut(&H::World, H::Node) -> bool
        + Send
        + Sync
        + 'static,
        apply: impl Fn(&mut H::World, H::Node) + Send + Sync + 'static,
    ) -> Self {
        self.ui.records.bindings.entry(self.node).or_default().push(
            Binding {
                changed: Box::new(changed),
                apply: Box::new(apply),
            },
        );
        self
    }
}
