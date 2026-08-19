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
/// `FnMut` because a predicate usually diffs against what it last saw,
/// and that value has to live somewhere. It must be called exactly
/// once per flush: a stateful predicate consumes its own signal.
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
/// It takes the whole table rather than one element, because only the
/// closure still knows which type to ask for: it was made where that
/// was still in hand, in [`ElementMut::bind`](crate::ui::ElementMut::bind).
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
/// A column per element type, so an element comes back as itself: no
/// erasure to undo, and asking for the wrong type is a miss rather
/// than a panic.
pub type Elements<H> = TypeTable<<H as Host>::Node>;

/// What a build registers as it runs, kept beside the world so both
/// can be borrowed at once.
pub struct Records<H: Host> {
    /// Keyed by the whole walk, so two fields of one element are two
    /// bindings and binding a field twice replaces rather than
    /// doubles.
    pub(crate) bindings: HashMap<(H::Node, FieldId), Binding<H>>,
    /// Keyed like the bindings, and one per field: two overlays on one
    /// field would each be the last word.
    pub(crate) lanes: Lanes<H>,
    pub(crate) elements: Elements<H>,
    /// Which nodes have a row in `mounts`. The table is keyed by type
    /// as well as node, so there is no way to ask it what it holds
    /// without naming a type: this is how a sweep knows what to drop.
    pub(crate) element_nodes: HashSet<H::Node>,
    pub(crate) store: Store<H>,
    /// Watchers declared during a build. They cannot go straight into
    /// the kernel's list, which is mid-iteration, and must not run
    /// until the next flush anyway.
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
    /// Where every `#[elem(child)]` field's node is recorded. The field
    /// itself stays private, so a caller that only wants to resolve
    /// one goes through [`Store::child`] instead of reaching in here.
    pub fn store(&self) -> &Store<H> {
        &self.store
    }

    /// As [`store`](Self::store), for `#[derive(Element)]`'s own
    /// generated code and anything else driving
    /// [`Element`](crate::element::Element) by hand - see its
    /// `build`/`patch`/`despawn`.
    pub fn store_mut(&mut self) -> &mut Store<H> {
        &mut self.store
    }

    /// The store and the lanes together, for
    /// [`Build::new`](crate::ui::Build::new) - what `#[derive(Element)]`'s
    /// own generated `build` reaches for rather than
    /// [`store_mut`](Self::store_mut) and a lane accessor called
    /// separately, which would each want their own exclusive borrow
    /// of `self` at once.
    #[doc(hidden)]
    pub fn build_parts(&mut self) -> (&mut Lanes<H>, &mut Store<H>) {
        (&mut self.lanes, &mut self.store)
    }
}
