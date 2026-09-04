//! What a build registers as it runs: watchers, bindings, transitions,
//! and the elements themselves - kept beside the world so both can be
//! borrowed at once.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::any::TypeId;
use core::hash::{Hash, Hasher};

use hashbrown::HashMap;
use typarena::type_table::TypeTable;

use crate::anim::{AnimTable, Registrar};
use crate::host::Host;
use crate::lenz::FieldId;
use crate::store::Store;
use crate::transition::TransitionTable;
use crate::ui::Ui;
use crate::world_node::WorldNodeRef;

/// A node and one of its fields, as a map key.
pub(crate) struct FieldKey<H: Host> {
    pub(crate) node: H::Node,
    pub(crate) field: FieldId,
}

impl<H: Host> FieldKey<H> {
    pub(crate) fn new(node: H::Node, field: FieldId) -> Self {
        Self { node, field }
    }
}

impl<H: Host> Clone for FieldKey<H> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<H: Host> Copy for FieldKey<H> {}

impl<H: Host> PartialEq for FieldKey<H> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node && self.field == other.field
    }
}

impl<H: Host> Eq for FieldKey<H> {}

impl<H: Host> Hash for FieldKey<H> {
    fn hash<S: Hasher>(&self, state: &mut S) {
        self.node.hash(state);
        self.field.hash(state);
    }
}

/// Predicate over a node's world, polled once per flush.
///
/// Must be called exactly once per flush. A stateful predicate
/// consumes its own signal.
pub trait ChangedFn<H: Host>:
    for<'w> FnMut(WorldNodeRef<'w, H>) -> bool + Send + Sync + 'static
{
}

impl<H: Host, F> ChangedFn<H> for F where
    F: for<'w> FnMut(WorldNodeRef<'w, H>) -> bool
        + Send
        + Sync
        + 'static
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
type BoxedChanged<H> =
    Box<dyn for<'w> FnMut(WorldNodeRef<'w, H>) -> bool + Send + Sync>;
type BoxedBuild<H> =
    Box<dyn for<'a> Fn(&mut Ui<'a, H>) + Send + Sync>;

/// Writes one field of a mounted element, then pushes it out.
///
/// Takes both tables whole: only the closure knows which type to ask
/// for.
type BoxedApply<H> = Box<
    dyn Fn(
            &mut ElementTable<H>,
            &mut TransitionTable<H>,
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
pub type ElementTable<H> = TypeTable<<H as Host>::Node>;

/// What a build registers as it runs, kept beside the world so both
/// can be borrowed at once.
pub struct Records<H: Host> {
    /// Keyed by the whole walk. Binding a field twice replaces it.
    pub(crate) bindings: HashMap<FieldKey<H>, Binding<H>>,
    /// Keyed like `bindings`, one per field.
    pub(crate) transitions: TransitionTable<H>,
    /// Tag-driven transitions, and what a build registered for them.
    pub(crate) anim: AnimTable<H>,
    pub(crate) elements: ElementTable<H>,
    /// The element type on each node with a row in `elements`. Lets a
    /// sweep know what to drop without asking `elements` what it
    /// holds, and names the type whose animated fields to re-resolve.
    pub(crate) element_nodes: HashMap<H::Node, TypeId>,
    pub(crate) store: Store<H>,
    /// Watchers declared during a build, held until the next flush.
    pub(crate) spawned: Vec<Watcher<H>>,
}

impl<H: Host> Default for Records<H> {
    fn default() -> Self {
        Self {
            bindings: HashMap::new(),
            transitions: TransitionTable::default(),
            anim: AnimTable::default(),
            elements: ElementTable::<H>::new(),
            element_nodes: HashMap::new(),
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

    /// The transition table and the store together, borrowed at once.
    #[doc(hidden)]
    pub fn build_parts(
        &mut self,
    ) -> (&mut TransitionTable<H>, &mut Store<H>) {
        (&mut self.transitions, &mut self.store)
    }

    /// Register element type `kind`'s animated fields, on its first
    /// build. Later calls for the same type do nothing.
    #[doc(hidden)]
    pub fn register_anim(
        &mut self,
        kind: TypeId,
        fields: impl FnOnce(&mut Registrar<'_, H>),
    ) {
        self.anim.register(kind, fields);
    }
}
