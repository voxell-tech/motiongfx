//! What a build registers as it runs: watchers, bindings, lanes, and
//! the elements themselves - kept beside the world so both can be
//! borrowed at once.

use alloc::boxed::Box;
use alloc::vec::Vec;

use hashbrown::{HashMap, HashSet};
use typarena::type_table::TypeTable;

use crate::host::Host;
use crate::lanes::Lanes;
use crate::lenz::FieldId;
use crate::store::Store;
use crate::ui::Ui;

/// Predicate over the world, polled once per flush.
///
/// Must be called exactly once per flush. A stateful predicate
/// consumes its own signal.
pub trait ChangedFn<H: Host>:
    FnMut(&H::World, H::Node) -> bool + Send + Sync + 'static
{
}

impl<H: Host, F> ChangedFn<H> for F where
    F: FnMut(&H::World, H::Node) -> bool + Send + Sync + 'static
{
}

/// Rebuilds the subtree under a node.
pub trait BuildFn<H: Host>:
    for<'a> Fn(&mut Ui<'a, H>) + Send + Sync + 'static
{
}

impl<H: Host, F> BuildFn<H> for F where
    F: for<'a> Fn(&mut Ui<'a, H>) + Send + Sync + 'static
{
}

// The boxed forms the kernel stores.
type BoxedChanged<H> = Box<
    dyn FnMut(&<H as Host>::World, <H as Host>::Node) -> bool
        + Send
        + Sync,
>;
type BoxedBuild<H> =
    Box<dyn for<'a> Fn(&mut Ui<'a, H>) + Send + Sync>;

/// Writes one field of a mounted element, then pushes it out.
///
/// Takes the whole table: only the closure knows which type to ask
/// for.
type BoxedApply<H> = Box<
    dyn Fn(
            &mut Elements<H>,
            &mut <H as Host>::World,
            <H as Host>::Node,
            &mut Store<H>,
            &<H as Host>::Theme,
        ) + Send
        + Sync,
>;

/// A field binding: when `changed` fires, the field is written and
/// patched.
pub struct Binding<H: Host> {
    pub(crate) changed: BoxedChanged<H>,
    pub(crate) apply: BoxedApply<H>,
}

/// A watcher rooted at a node.
pub struct Watcher<H: Host> {
    pub(crate) root: H::Node,
    pub(crate) changed: BoxedChanged<H>,
    pub(crate) build: BoxedBuild<H>,
}

/// The elements the kernel built and still owns, by the node each one
/// took.
///
/// A column per element type, so asking for the wrong type is a miss,
/// not a panic.
pub type Elements<H> = TypeTable<<H as Host>::Node>;

/// What a build registers as it runs, kept beside the world so both
/// can be borrowed at once.
pub struct Records<H: Host> {
    /// Keyed by the whole walk. Binding a field twice replaces it.
    pub(crate) bindings: HashMap<(H::Node, FieldId), Binding<H>>,
    /// Keyed like `bindings`, one per field.
    pub(crate) lanes: Lanes<H>,
    pub(crate) elements: Elements<H>,
    /// Which nodes have a row in `elements`. Lets a sweep know what
    /// to drop without asking `elements` what it holds.
    pub(crate) element_nodes: HashSet<H::Node>,
    pub(crate) store: Store<H>,
    /// Watchers declared during a build, held until the next flush.
    pub(crate) spawned: Vec<Watcher<H>>,
}

impl<H: Host> Default for Records<H> {
    fn default() -> Self {
        Self {
            bindings: HashMap::new(),
            lanes: Lanes::default(),
            elements: Elements::<H>::new(),
            element_nodes: HashSet::new(),
            store: Store::new(),
            spawned: Vec::new(),
        }
    }
}

impl<H: Host> Records<H> {
    /// Where every `#[elem(child)]` field's node is recorded.
    pub fn store(&self) -> &Store<H> {
        &self.store
    }

    /// As [`store`](Self::store), mutable.
    pub fn store_mut(&mut self) -> &mut Store<H> {
        &mut self.store
    }

    /// The store and the lanes together, borrowed at once.
    #[doc(hidden)]
    pub fn build_parts(&mut self) -> (&mut Lanes<H>, &mut Store<H>) {
        (&mut self.lanes, &mut self.store)
    }
}
