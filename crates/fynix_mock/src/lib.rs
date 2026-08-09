//! A mock of the fynix element model.

#![no_std]

extern crate alloc;

// Lets the derive emit `::fynix_mock::...` everywhere, including here.
extern crate self as fynix_mock;

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::host::Host;
use crate::ui::{BuildFn, ChangedFn, Records, Ui, Watcher};

mod elem;
pub mod element;
pub mod host;
pub mod lenz;
pub mod store;
pub mod style;
pub mod ui;

/// Writes the `Default` an element starts from, before a style and then
/// a call site have had their say.
///
/// Nothing about it is UI: it applies to any struct or enum whose
/// fields want a default other than their own.
///
/// A field says what it starts as in one of these ways, none of which
/// name the field's type:
///
/// ```
/// use fynix_mock::OverrideDefault;
/// #[derive(Default)]
/// struct Font { size: u32, weight: u32 }
///
/// #[derive(OverrideDefault)]
/// enum Weight {
///     Thin,
///     #[default]
///     Bold,
/// }
///
/// #[derive(OverrideDefault)]
/// struct Label {
///     #[default(size: 24, weight: 400)] // its own default, overridden
///     font: Font,
///     #[default(0: 1, 1: 2)]            // the same, by index
///     origin: (u32, u32),
///     #[default(_, 8, ..)]              // the same, as the pattern it
///                                       // looks like: `_` and `..`
///                                       // keep what the default had
///     margin: (u32, u32, u32),
///     #[default(::Thin)]                // a variant of the field's type
///     weight: Weight,
///     #[default(..)]                    // an `Option`, filled
///     icon: Option<Font>,
/// }
///
/// let label = Label::default();
///
/// assert_eq!((label.font.size, label.font.weight), (24, 400));
/// assert_eq!(label.origin, (1, 2));
/// assert_eq!(label.margin, (0, 8, 0));
/// assert!(matches!(label.weight, Weight::Thin));
/// assert!(label.icon.is_some());
/// ```
///
/// Anything else is the value, whole: `#[default(px(4))]`. Braces say
/// so outright, for a value that would otherwise read as one of the
/// above: `#[default({ ::core::f32::consts::PI })]`.
///
/// An enum starts in the variant marked `#[default]`, and that
/// variant's own fields take the attribute as any other field does.
pub use fynix_mock_macros::OverrideDefault;

/// Owns every watcher and binding, and the tree they maintain.
pub struct Fynix<H: Host> {
    watchers: Vec<Watcher<H>>,
    records: Records<H>,
}

impl<H: Host> Default for Fynix<H> {
    fn default() -> Self {
        Self {
            watchers: Vec::new(),
            records: Records::default(),
        }
    }
}

impl<H: Host> Fynix<H> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuild the subtree under `root` whenever `changed` fires.
    ///
    /// The bootstrap: every other watcher is declared inside a build,
    /// through [`ElementMut::watch`](crate::ui::ElementMut::watch).
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

    /// How many watchers the kernel is holding.
    ///
    /// One per node it may rebuild, so this is what grows if a root
    /// goes without its watcher going too.
    pub fn watcher_len(&self) -> usize {
        self.watchers.len()
    }

    /// How many elements the kernel is holding.
    ///
    /// Every one is a node it built and has not swept, so this is what
    /// grows if a teardown ever misses.
    pub fn element_len(&self) -> usize {
        self.records.element_nodes.len()
    }

    /// How many bindings the kernel is holding.
    ///
    /// One per field bound to a live node, so this is what grows if a
    /// rebuild ever leaves its old bindings behind.
    pub fn binding_len(&self) -> usize {
        self.records.bindings.len()
    }

    /// Stop watching `root`. Its nodes are left alone.
    pub fn unwatch(&mut self, root: H::Node) {
        self.watchers.retain(|watcher| watcher.root != root);
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

            clear_children::<H>(world, watcher.root);
            let mut ui = Ui::new(world, watcher.root, records);
            (watcher.build)(&mut ui);
        }

        watchers.append(&mut records.spawned);
        watchers.retain(|watcher| H::exists(world, watcher.root));

        // Everything the kernel keeps is keyed on a node, and a node
        // can go at any time: a rebuild above cleared one, or the app
        // despawned another out from under us. One sweep covers both,
        // and has to come before anything is applied to a dead handle.
        records
            .bindings
            .retain(|(node, _), _| H::exists(world, *node));
        records.store.prune(world);

        // The table is keyed by type as well as node, so it cannot be
        // asked what it holds. `element_nodes` is the list to sweep,
        // and dropping the row takes the element whatever type it is.
        records.element_nodes.retain(|node| {
            let alive = H::exists(world, *node);
            if !alive {
                records.elements.remove_row(node);
            }
            alive
        });

        let Records {
            bindings,
            elements,
            store,
            ..
        } = records;

        for ((node, _), binding) in bindings.iter_mut() {
            if !(binding.changed)(world, *node) {
                continue;
            }
            (binding.apply)(elements, world, *node, store);
        }
    }
}

/// Despawn the kernel's children of `root`.
///
/// The host takes each subtree with it, and the sweep in
/// [`Fynix::flush`] drops whatever those nodes left in the records:
/// the same job for a rebuild as for a node the app removed itself.
fn clear_children<H: Host>(world: &mut H::World, root: H::Node) {
    for child in H::children(world, root) {
        H::despawn(world, child);
    }
}
