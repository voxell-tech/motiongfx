//! The builder.
//!
//! [`Ui`] holds `&mut World` and builds as it goes, so a builder gets
//! each element's node the moment it makes one — letting a binding
//! refer to a sibling or parent by handle. The cost: a builder can't
//! hold a world borrow across a build, so collect what you need first.
//!
//! Everything the kernel makes is an element. There is no way to spawn
//! a bare node, because a node nothing owns could never be patched.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::marker::PhantomData;

use hashbrown::{HashMap, HashSet};
use typarena::type_table::TypeTable;

use crate::element::Element;
use crate::host::Host;
use crate::lenz::{Cursor, FieldId, FieldPath, Identity};
use crate::store::Store;

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
/// was still in hand, in [`ElementMut::bind`].
type BoxedApply<H> = Box<
    dyn Fn(
            &mut Elements<H>,
            &mut <H as Host>::World,
            <H as Host>::Node,
            &mut Store<H>,
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
            elements: Elements::<H>::new(),
            element_nodes: HashSet::new(),
            store: Store::new(),
            spawned: Vec::new(),
        }
    }
}

/// Builds elements under a parent and records their reactivity.
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
    /// building: the borrow cannot outlive the next builder call.
    pub fn world(&self) -> &H::World {
        self.world
    }

    /// The node these children are being built under.
    pub fn parent(&self) -> H::Node {
        self.parent
    }

    /// Build `element` and everything beneath it.
    ///
    /// The kernel keeps the element, because patching a field later
    /// means reading it back.
    pub fn element<E>(
        &mut self,
        element: E,
    ) -> ElementMut<'_, 'a, H, E>
    where
        E: Element<H> + Send + Sync,
    {
        let node = element.build(
            self.world,
            self.parent,
            &mut self.records.store,
        );
        self.records.elements.insert(node, element);
        self.records.element_nodes.insert(node);

        ElementMut {
            ui: self,
            node,
            element: PhantomData,
        }
    }
}

/// A freshly built element, for chaining children and bindings.
///
/// It remembers which element it built, so a binding can only be made
/// from a walk that starts there.
pub struct ElementMut<'u, 'a, H: Host, E: Element<H>> {
    ui: &'u mut Ui<'a, H>,
    node: H::Node,
    element: PhantomData<fn() -> E>,
}

impl<H: Host, E: Element<H>> ElementMut<'_, '_, H, E> {
    /// This element's node. Capture it to bind a *later* element
    /// against this one.
    pub fn id(&self) -> H::Node {
        self.node
    }

    /// Rebuild this element's children whenever `changed` fires. First
    /// runs on the next flush.
    ///
    /// Use this *or* [`Self::with`], not both: a fire clears whatever
    /// children the node has.
    pub fn watch(
        self,
        changed: impl ChangedFn<H>,
        build: impl BuildFn<H>,
    ) -> Self {
        self.ui.records.spawned.push(Watcher {
            root: self.node,
            changed: Box::new(changed),
            build: Box::new(build),
        });
        self
    }

    /// Build children under this element.
    pub fn with(self, f: impl FnOnce(&mut Ui<'_, H>)) -> Self {
        let mut child =
            Ui::new(self.ui.world, self.node, self.ui.records);
        f(&mut child);
        self
    }

    /// Write `value` into `cursor` whenever `changed` fires, then push
    /// that one field out to the backend.
    ///
    /// The walk has to start at this element, so a binding cannot be
    /// made against the wrong one. Both halves come from that single
    /// walk, so the write and the patch cannot address different
    /// fields, and an absent `Option` along the way skips both rather
    /// than panicking.
    pub fn bind<P>(
        self,
        field: impl FnOnce(Cursor<Identity<E>>) -> Cursor<P>,
        changed: impl ChangedFn<H>,
        value: impl Fn(&H::World, H::Node) -> P::Target
        + Send
        + Sync
        + 'static,
    ) -> Self
    where
        P: FieldPath<Source = E>,
        P::Target: Send + Sync,
    {
        // The walk starts here, so the caller never names the element
        // again, and cannot name a different one.
        let cursor = field(Cursor::new());

        let accessor = cursor.accessor();
        let hops = cursor.hops();

        let apply = move |elements: &mut Elements<H>,
                          world: &mut H::World,
                          node: H::Node,
                          store: &mut Store<H>| {
            let new = value(world, node);

            // `E` is still in hand here, so the element comes back as
            // itself. A node holding something else simply misses.
            let Some(element) = elements.get_mut::<E>(&node) else {
                return;
            };
            let Some(field) = (accessor.get_mut)(element) else {
                return;
            };
            *field = new;

            element.patch(world, node, &hops, store);
        };

        self.ui.records.bindings.insert(
            (self.node, cursor.key()),
            Binding {
                changed: Box::new(changed),
                apply: Box::new(apply),
            },
        );
        self
    }
}
