//! A mock of the fynix element model.

#![no_std]

extern crate alloc;

// Lets the derive emit `::fynix::...` everywhere, including here.
extern crate self as fynix;

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::host::Host;
use crate::lenz::{Cursor, FieldPath, Identity};
use crate::records::{BuildFn, ChangedFn, Records, Watcher};
use crate::ui::Ui;

pub mod composer;
mod elem;
pub mod element;
pub mod host;
pub mod lanes;
pub mod lenz;
pub mod records;
pub mod store;
pub mod style;
pub mod transition;
pub mod ui;
pub mod world_node;

pub use crate::world_node::{WorldNodeMut, WorldNodeRef};

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
/// use fynix::OverrideDefault;
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
pub use fynix_macros::OverrideDefault;

/// Owns every watcher and binding, and the tree they maintain.
pub struct Fynix<H: Host> {
    watchers: Vec<Watcher<H>>,
    records: Records<H>,
    /// Not read from `World`.
    theme: H::Theme,
    /// Set by [`Self::theme_mut`], cleared by [`Self::flush`]. Forces
    /// a full rebuild on the next flush, since every element already
    /// built has the old theme baked in.
    theme_dirty: bool,
}

impl<H: Host> Fynix<H> {
    /// Starts empty, themed with `theme`.
    pub fn new(theme: H::Theme) -> Self {
        Self {
            watchers: Vec::new(),
            records: Records::default(),
            theme,
            theme_dirty: false,
        }
    }

    /// The current theme.
    pub fn theme(&self) -> &H::Theme {
        &self.theme
    }

    /// The theme, to edit in place. Any edit rebuilds the whole tree
    /// on the next [`Self::flush`].
    pub fn theme_mut(&mut self) -> &mut H::Theme {
        self.theme_dirty = true;
        &mut self.theme
    }

    /// Rebuild the subtree under `root` whenever `changed` fires.
    /// Mirrors [`ElementMut::watch`](crate::ui::ElementMut::watch).
    ///
    /// This is the bootstrap watcher. Every other one is added
    /// through `ElementMut::watch` inside a build.
    pub fn watch(
        &mut self,
        root: H::Node,
        changed: impl ChangedFn<H>,
        build: impl BuildFn<H>,
        world: &mut H::World,
    ) {
        let mut changed = changed;
        if changed(WorldNodeRef::new(world, root)) {
            clear_children::<H>(world, root);
            let mut ui =
                Ui::new(world, root, &mut self.records, &self.theme);
            build(&mut ui);
        }

        self.watchers.push(Watcher {
            root,
            changed: Box::new(changed),
            build: Box::new(build),
        });
    }

    /// How many watchers the kernel is holding.
    pub fn watcher_len(&self) -> usize {
        self.watchers.len()
    }

    /// How many elements the kernel is holding.
    pub fn element_len(&self) -> usize {
        self.records.element_nodes.len()
    }

    /// The current value of `E` built on `node`, if the kernel still
    /// has one. The same value
    /// [`ElementMut::bind`](crate::ui::ElementMut::bind) patches on
    /// change, so this always reads the latest.
    pub fn element<E: 'static>(&self, node: H::Node) -> Option<&E> {
        self.records.elements.get(&node)
    }

    /// How many bindings the kernel is holding.
    pub fn binding_len(&self) -> usize {
        self.records.bindings.len()
    }

    /// Stop watching `root`. Its nodes are left alone.
    pub fn unwatch(&mut self, root: H::Node) {
        self.watchers.retain(|watcher| watcher.root != root);
    }

    /// Run every watcher and binding whose predicate fires.
    pub fn flush(&mut self, world: &mut H::World) {
        // Taken, not read, so a `theme_mut` call during this flush
        // still schedules another rebuild.
        let retheme = core::mem::take(&mut self.theme_dirty);

        // Split so `records` stays writable while `watchers` is
        // borrowed by the loop below.
        let Self {
            watchers,
            records,
            theme,
            ..
        } = self;

        for watcher in watchers.iter_mut() {
            // Checked per watcher: an earlier rebuild this flush can
            // despawn a later watcher's root.
            if !H::exists(world, watcher.root) {
                continue;
            }
            // Called even when `retheme` forces the rebuild anyway.
            // Some `changed` closures are one-shot; skipping the call
            // would leave them armed for a later flush.
            let changed = (watcher.changed)(WorldNodeRef::new(
                world,
                watcher.root,
            ));
            if !retheme && !changed {
                continue;
            }

            clear_children::<H>(world, watcher.root);
            let mut ui = Ui::new(world, watcher.root, records, theme);
            (watcher.build)(&mut ui);
        }

        watchers.append(&mut records.spawned);
        watchers.retain(|watcher| H::exists(world, watcher.root));

        // A node can die at any time: a rebuild above cleared one, or
        // the app despawned another. Sweep both before touching any
        // dead handle.
        records
            .bindings
            .retain(|(node, _), _| H::exists(world, *node));
        records.lanes.retain(|node| H::exists(world, node));
        records.store.prune(world);

        // `elements` is keyed by type as well as node, so it cannot
        // say what it holds. `element_nodes` is the list to sweep.
        records.element_nodes.retain(|node| {
            let alive = H::exists(world, *node);
            if !alive {
                records.elements.remove_row(node);
            }
            alive
        });

        let Records {
            bindings,
            lanes,
            elements,
            store,
            ..
        } = records;

        for ((node, _), binding) in bindings.iter_mut() {
            if !(binding.changed)(WorldNodeRef::new(world, *node)) {
                continue;
            }
            (binding.apply)(elements, world, *node, store, theme);
        }

        // After the bindings, so a lane gets the last word over the
        // base they left.
        let delta = H::delta(world);

        for (node, lane) in lanes.iter_mut() {
            lane.advance(
                delta,
                elements,
                WorldNodeMut::new(world, node),
                store,
                theme,
            );
        }
    }

    /// Point a transitioning field at `target`, or release it back to
    /// its base with `None`. Aiming a field with no lane does
    /// nothing.
    pub fn aim<E, P>(
        &mut self,
        node: H::Node,
        field: impl FnOnce(Cursor<Identity<E>>) -> Cursor<P>,
        target: Option<P::Target>,
    ) where
        E: 'static,
        P: FieldPath<Source = E>,
        P::Target: 'static,
    {
        let key = field(Cursor::new()).key();

        if let Some(lane) = self.records.lanes.get_mut(node, key) {
            let mut target = target;
            lane.aim(&mut target);
        }
    }

    /// How many transitioning fields the kernel is holding.
    pub fn lane_len(&self) -> usize {
        self.records.lanes.len()
    }
}

/// Despawn the kernel's children of `root`. The sweep in
/// [`Fynix::flush`] then drops whatever those nodes left behind.
pub(crate) fn clear_children<H: Host>(
    world: &mut H::World,
    root: H::Node,
) {
    for child in H::children(world, root) {
        H::despawn(world, child);
    }
}
