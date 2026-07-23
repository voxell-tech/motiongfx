#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

pub mod host;
pub mod ui;

use alloc::boxed::Box;
use alloc::vec::Vec;

use hashbrown::HashMap;

pub use host::Host;
pub use ui::{
    ApplyFn, Binding, BuildFn, ChangedFn, NodeMut, Records, Ui,
};

use ui::Watcher;

/// Owns every watcher and binding, and the tree they maintain.
pub struct Kernel<H: Host> {
    watchers: Vec<Watcher<H>>,
    records: Records<H>,
}

impl<H: Host> Default for Kernel<H> {
    fn default() -> Self {
        Self {
            watchers: Vec::new(),
            records: Records::default(),
        }
    }
}

impl<H: Host> Kernel<H> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuild the subtree under `root` whenever `changed` fires.
    ///
    /// The bootstrap: every other watcher is declared inside a build,
    /// through [`NodeMut::watch`].
    pub fn watch(
        &mut self,
        root: H::Node,
        changed: impl ChangedFn<H>,
        build: impl BuildFn<H>,
    ) {
        self.watchers.push(Watcher {
            root,
            changed: Box::new(changed),
            build: Box::new(build),
        });
    }

    /// Stop watching `root`. Its nodes are left alone.
    pub fn unwatch(&mut self, root: H::Node) {
        self.watchers.retain(|watcher| watcher.root != root);
    }

    /// Bind fields on an existing `node`, not one the kernel spawned.
    /// The kernel still prunes it when `node` is despawned.
    pub fn bind(&mut self, node: H::Node, binding: Binding<H>) {
        self.records.bindings.entry(node).or_default().push(binding);
    }

    /// Run every watcher and binding whose predicate fires.
    pub fn flush(&mut self, world: &mut H::World) {
        // Split the borrow: a build writes into `records` while
        // `watchers` is still borrowed by the loop.
        let Self { watchers, records } = self;

        for watcher in watchers.iter_mut() {
            // Per watcher, not once up front: an earlier rebuild in
            // this same flush can despawn a later watcher's root.
            if !H::exists(world, watcher.root) {
                continue;
            }
            if !(watcher.changed)(world, watcher.root) {
                continue;
            }

            clear_children::<H>(
                world,
                watcher.root,
                &mut records.bindings,
            );
            let mut ui = Ui::new(world, watcher.root, records);
            (watcher.build)(&mut ui);
        }

        watchers.append(&mut records.spawned);
        watchers.retain(|watcher| H::exists(world, watcher.root));

        // Nodes can also be despawned by the app out from under a
        // binding. Applying to a dead handle is the host's problem to
        // panic about, so prune first.
        records.bindings.retain(|node, _| H::exists(world, *node));

        for (node, list) in records.bindings.iter_mut() {
            for binding in list.iter_mut() {
                if (binding.changed)(world, *node) {
                    (binding.apply)(world, *node);
                }
            }
        }
    }
}

/// Despawn the kernel's children of `root`, dropping their bindings.
fn clear_children<H: Host>(
    world: &mut H::World,
    root: H::Node,
    bindings: &mut HashMap<H::Node, Vec<Binding<H>>>,
) {
    for child in H::children(world, root) {
        drop_subtree::<H>(world, child, bindings);
        H::despawn(world, child);
    }
}

/// Forget bindings for `node` and everything beneath it. Without this
/// a rebuild leaks a binding per node, and stale ones keep firing
/// against handles the host has already freed.
fn drop_subtree<H: Host>(
    world: &H::World,
    node: H::Node,
    bindings: &mut HashMap<H::Node, Vec<Binding<H>>>,
) {
    bindings.remove(&node);
    for child in H::children(world, node) {
        drop_subtree::<H>(world, child, bindings);
    }
}
