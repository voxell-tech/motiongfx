//! The builder.
//!
//! [`Ui`] holds `&mut World` and builds as it goes, so a builder gets
//! each element's node the moment it makes one, letting a binding
//! refer to a sibling or parent by handle. The cost: a builder can't
//! hold a world borrow across a build, so collect what you need first.
//!
//! Everything the kernel makes is an element. There is no way to spawn
//! a bare node, because a node nothing owns could never be patched.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::any::Any;
use core::marker::PhantomData;

use hashbrown::{HashMap, HashSet};
use typarena::type_table::TypeTable;

use crate::composer::Composer;
use crate::element::Element;
use crate::host::Host;
use crate::lenz::{Accessor, Cursor, FieldId, FieldPath, Identity};
use crate::store::Store;
use crate::style::StyledElem;
use crate::transition::Transition;

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

/// A field on its way somewhere, kept beside the element rather than
/// in it.
///
/// The element keeps the value the cascade gave it, the *base*; a lane
/// keeps what the backend is showing, and pushes it through the
/// element's own patch by swapping it in for the length of one call.
pub(crate) trait Lane<H: Host>: Send + Sync {
    /// Point this lane somewhere new: a boxed `Option<T>`, `None` to
    /// release back to the base. Another type is ignored.
    fn aim(&mut self, target: &mut dyn Any);

    /// Advance by `dt` and push what it reached. `false` once it has
    /// nothing left to say and the base is what shows.
    fn advance(
        &mut self,
        dt: f32,
        elements: &mut Elements<H>,
        world: &mut H::World,
        node: H::Node,
        store: &mut Store<H>,
    ) -> bool;
}

/// One lane, with the types it was made from still in hand.
struct Travel<H: Host, E, T: 'static> {
    accessor: Accessor<E, T>,
    hops: Vec<FieldId>,
    transition: Transition<T>,
    /// What the backend is showing.
    shown: T,
    /// Where this leg started.
    from: T,
    /// Where it is heading: the target, or the base once released.
    heading: T,
    /// The override, or `None` while released.
    target: Option<T>,
    elapsed: f32,
    host: PhantomData<fn() -> H>,
}

impl<H, E, T> Lane<H> for Travel<H, E, T>
where
    H: Host,
    E: Element<H> + Send + Sync,
    T: PartialEq + Clone + Send + Sync + 'static,
{
    fn aim(&mut self, target: &mut dyn Any) {
        if let Some(target) = target.downcast_mut::<Option<T>>() {
            self.target = target.take();
        }
    }

    fn advance(
        &mut self,
        dt: f32,
        elements: &mut Elements<H>,
        world: &mut H::World,
        node: H::Node,
        store: &mut Store<H>,
    ) -> bool {
        let Some(element) = elements.get_mut::<E>(&node) else {
            return false;
        };
        let Some(base) = (self.accessor.get)(element) else {
            return false;
        };

        // The base moves under a running leg whenever a binding writes
        // it, so where this is heading is worked out afresh each time.
        let heading =
            self.target.clone().unwrap_or_else(|| base.clone());

        if heading != self.heading {
            self.from = self.shown.clone();
            self.heading = heading;
            self.elapsed = 0.0;
        }

        if self.shown == self.heading {
            // Released and arrived: the base already shows.
            if self.target.is_none() {
                return false;
            }
        } else {
            self.elapsed += dt;
            self.shown = if self.transition.done(self.elapsed) {
                self.heading.clone()
            } else {
                (self.transition.lerp)(
                    &self.from,
                    &self.heading,
                    self.transition.at(self.elapsed),
                )
            };
        }

        // Pushed even when it did not move, so that a binding writing
        // the base this same flush cannot be the last word.
        let Some(field) = (self.accessor.get_mut)(element) else {
            return false;
        };
        let base = core::mem::replace(field, self.shown.clone());

        element.patch(world, node, &self.hops, store);

        if let Some(field) = (self.accessor.get_mut)(element) {
            *field = base;
        }
        true
    }
}

/// Every field currently travelling rather than snapped to its base,
/// keyed like the bindings: one per field, so a second lane on the
/// same one replaces rather than doubles.
///
/// Opaque on purpose: [`Lane`] is `pub(crate)`, so this is what a
/// borrow of the table looks like to anything outside this crate -
/// [`Draw`] holds one directly, the same way it holds a [`Store`],
/// without either leaking what a lane actually is.
pub struct Lanes<H: Host>(
    HashMap<(H::Node, FieldId), Box<dyn Lane<H>>>,
);

impl<H: Host> Default for Lanes<H> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<H: Host> Lanes<H> {
    pub(crate) fn insert(
        &mut self,
        node: H::Node,
        key: FieldId,
        lane: Box<dyn Lane<H>>,
    ) {
        self.0.insert((node, key), lane);
    }

    pub(crate) fn get_mut(
        &mut self,
        node: H::Node,
        key: FieldId,
    ) -> Option<&mut Box<dyn Lane<H>>> {
        self.0.get_mut(&(node, key))
    }

    pub(crate) fn retain(
        &mut self,
        mut keep: impl FnMut(H::Node) -> bool,
    ) {
        self.0.retain(|(node, _), _| keep(*node));
    }

    pub(crate) fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (H::Node, &mut Box<dyn Lane<H>>)> {
        self.0.iter_mut().map(|((node, _), lane)| (*node, lane))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The shared insides of [`ElementMut::transition_from`] and
/// [`Draw::transition_from`] - only where the lane table comes from
/// differs between them.
fn insert_lane<H, E, P>(
    lanes: &mut Lanes<H>,
    node: H::Node,
    field: impl FnOnce(Cursor<Identity<E>>) -> Cursor<P>,
    base: P::Target,
    transition: Transition<P::Target>,
) where
    H: Host,
    E: Element<H> + Send + Sync,
    P: FieldPath<Source = E>,
    P::Target: PartialEq + Clone + Send + Sync,
{
    let cursor = field(Cursor::new());
    let accessor = cursor.accessor();

    lanes.insert(
        node,
        cursor.key(),
        Box::new(Travel::<H, E, P::Target> {
            accessor,
            hops: cursor.hops(),
            transition,
            shown: base.clone(),
            from: base.clone(),
            heading: base,
            target: None,
            elapsed: 0.0,
            host: PhantomData,
        }),
    );
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
    /// [`Draw::new`](Draw::new) - what `#[derive(Element)]`'s own
    /// generated `build` reaches for rather than
    /// [`store_mut`](Self::store_mut) and a lane accessor called
    /// separately, which would each want their own exclusive borrow
    /// of `self` at once.
    #[doc(hidden)]
    pub fn draw_parts(&mut self) -> (&mut Lanes<H>, &mut Store<H>) {
        (&mut self.lanes, &mut self.store)
    }
}

/// Builds elements under a parent and records their reactivity.
pub struct Ui<'a, H: Host> {
    /// What the builders read and the applies write. Public, because
    /// a host's own extensions need it and a build is the one place
    /// holding it.
    pub world: &'a mut H::World,
    /// Borrowed from [`Fynix`](crate::Fynix)'s own field, never from
    /// `world` - see [`Host::Theme`].
    pub theme: &'a H::Theme,
    parent: H::Node,
    records: &'a mut Records<H>,
}

impl<'a, H: Host> Ui<'a, H> {
    /// Not for hand-written code: `#[derive(Element)]`'s own generated
    /// `build` is what constructs the `Ui` a
    /// [`build_fields`](crate::element::ElementVisual::build_fields)
    /// is handed, from the world and records it already has in scope.
    #[doc(hidden)]
    pub fn new(
        world: &'a mut H::World,
        parent: H::Node,
        records: &'a mut Records<H>,
        theme: &'a H::Theme,
    ) -> Self {
        Self {
            world,
            theme,
            parent,
            records,
        }
    }

    /// The node these children are being built under - or, from
    /// inside [`build_fields`](crate::element::ElementVisual::build_fields),
    /// the element's own node.
    pub fn parent(&self) -> H::Node {
        self.parent
    }

    /// The node a `#[elem(child)]` field of [`parent`](Self::parent) built,
    /// however many hops the path takes to reach it. See
    /// [`Store::child`].
    pub fn child<S, P>(
        &self,
        field: impl FnOnce(Cursor<Identity<S>>) -> Cursor<P>,
    ) -> Option<H::Node>
    where
        P: FieldPath<Source = S>,
    {
        self.records.store.child(self.parent, field)
    }

    /// Run a [`StyledElem`]'s cascade, then build what it left, and
    /// everything beneath it.
    ///
    /// What [`elem!`](crate::elem!) is for: the macro says how the
    /// element is described, and this says where it goes. The kernel
    /// keeps the element, because patching a field later means reading
    /// it back.
    pub fn elem<S, E>(
        &mut self,
        styled: S,
    ) -> ElementMut<'_, 'a, H, E>
    where
        S: StyledElem<Host = H, Element = E>,
        E: Element<H> + Send + Sync,
    {
        let element = styled.create(self.theme);

        let node = element.build(
            self.world,
            self.parent,
            self.records,
            self.theme,
        );
        self.records.elements.insert(node, element);
        self.records.element_nodes.insert(node);

        ElementMut {
            ui: self,
            node,
            element: PhantomData,
        }
    }

    /// Run a [`Composer`], and take back the root of what it built.
    ///
    /// Unlike [`elem`](Self::elem), what goes in is never stored -
    /// only what it built outlives the call. What comes back is the
    /// same [`ElementMut`] an element would give.
    pub fn compose<C>(
        &mut self,
        composer: C,
    ) -> ElementMut<'_, 'a, H, C::Element>
    where
        C: Composer<H>,
        C::Element: Element<H> + Send + Sync,
    {
        let node = composer.compose(self).node();

        ElementMut {
            ui: self,
            node,
            element: PhantomData,
        }
    }
}

/// What [`build_fields`](crate::element::ElementVisual::build_fields)
/// writes through: this element's own node, `world`, and `theme`,
/// plus the two tables a look wired at build time reaches for again -
/// [`child`](Self::child) and [`transition_from`](Self::transition_from).
///
/// Deliberately not [`ElementMut`]: `bind`/`watch`/`with` declare what
/// a node does once it exists, and a node running its own
/// `build_fields` has not finished existing yet - calling any of them
/// on itself, mid build, would not mean what it means anywhere else
/// they're reached for. Nor is it [`Records`] itself, or even both its
/// tables through one borrow of it: `#[elem(child)]`'s children keep
/// [`Store`] straight, and a lane keeps [`Lanes`] straight, without
/// `build_fields` ever seeing either name.
pub struct Draw<'a, H: Host, E: Element<H>> {
    pub world: &'a mut H::World,
    pub theme: &'a H::Theme,
    node: H::Node,
    lanes: &'a mut Lanes<H>,
    store: &'a mut Store<H>,
    element: PhantomData<fn() -> E>,
}

impl<'a, H: Host, E: Element<H>> Draw<'a, H, E> {
    /// Not for hand-written code: `#[derive(Element)]`'s own generated
    /// `build` is what constructs this, from the pieces of
    /// [`Records`] it already has in scope - see
    /// [`Records::draw_parts`].
    #[doc(hidden)]
    pub fn new(
        world: &'a mut H::World,
        node: H::Node,
        lanes: &'a mut Lanes<H>,
        store: &'a mut Store<H>,
        theme: &'a H::Theme,
    ) -> Self {
        Self {
            world,
            theme,
            node,
            lanes,
            store,
            element: PhantomData,
        }
    }

    /// This element's own node.
    pub fn id(&self) -> H::Node {
        self.node
    }

    /// The node an `#[elem(child)]` child took.
    ///
    /// `None` when the walk names a field that is not an element, or
    /// an `Option` child that is absent.
    pub fn child<P>(
        &self,
        field: impl FnOnce(Cursor<Identity<E>>) -> Cursor<P>,
    ) -> Option<H::Node>
    where
        P: FieldPath<Source = E>,
    {
        self.store.child(self.node, field)
    }

    /// As [`ElementMut::transition_from`]: a lane starting from `base`
    /// rather than a base read out of the kernel's own table, which
    /// this node has no entry in yet.
    pub fn transition_from<P>(
        &mut self,
        field: impl FnOnce(Cursor<Identity<E>>) -> Cursor<P>,
        base: P::Target,
        transition: Transition<P::Target>,
    ) -> &mut Self
    where
        E: Send + Sync,
        P: FieldPath<Source = E>,
        P::Target: PartialEq + Clone + Send + Sync,
    {
        insert_lane::<H, E, P>(
            self.lanes, self.node, field, base, transition,
        );
        self
    }
}

/// A typed, [`Copy`] handle to a node: what names an element once
/// there is no borrow of the [`Ui`] left to name it through.
///
/// The tag is what a later walk is checked against. `fn() -> E` keeps
/// the handle neutral on variance and auto traits while owning no
/// `E`.
pub struct ElementHandle<H: Host, E> {
    node: H::Node,
    element: PhantomData<fn() -> E>,
}

impl<H: Host, E> ElementHandle<H, E> {
    /// Tags `node` with the element it was built from.
    pub fn new(node: H::Node) -> Self {
        Self {
            node,
            element: PhantomData,
        }
    }

    /// The node itself, with the tag dropped.
    pub fn node(self) -> H::Node {
        self.node
    }
}

impl<H: Host, E> Clone for ElementHandle<H, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<H: Host, E> Copy for ElementHandle<H, E> {}

/// A freshly built element, for chaining children and bindings.
///
/// It remembers which element it built, so a binding can only be made
/// from a walk that starts there.
pub struct ElementMut<'u, 'a, H: Host, E: Element<H>> {
    pub ui: &'u mut Ui<'a, H>,
    node: H::Node,
    element: PhantomData<fn() -> E>,
}

impl<H: Host, E: Element<H>> ElementMut<'_, '_, H, E> {
    /// This element's node. Capture it to bind a *later* element
    /// against this one.
    pub fn id(&self) -> H::Node {
        self.node
    }

    /// This element as a handle, owning no borrow - what a
    /// [`Composer`] hands back.
    pub fn handle(&self) -> ElementHandle<H, E> {
        ElementHandle::new(self.node)
    }

    /// Rebuild this element's children whenever `changed` fires. First
    /// runs on the next flush.
    ///
    /// Use this *or* [`Self::with`], not both: a fire clears whatever
    /// children the node has.
    pub fn watch(
        &mut self,
        changed: impl ChangedFn<H>,
        build: impl BuildFn<H>,
    ) -> &mut Self {
        self.ui.records.spawned.push(Watcher {
            root: self.node,
            changed: Box::new(changed),
            build: Box::new(build),
        });
        self
    }

    /// The node an `#[elem(child)]` child took.
    ///
    /// For what the child owns rather than the element does: an
    /// observer on the button inside a tab fires for that button, and
    /// nothing above it ever sees the click. `None` when the walk
    /// names a field that is not an element, or an `Option` child that
    /// is absent.
    pub fn child<P>(
        &self,
        field: impl FnOnce(Cursor<Identity<E>>) -> Cursor<P>,
    ) -> Option<H::Node>
    where
        P: FieldPath<Source = E>,
    {
        self.ui.records.store.child(self.node, field)
    }

    /// Build children under this element.
    pub fn with(
        &mut self,
        f: impl FnOnce(&mut Ui<'_, H>),
    ) -> &mut Self {
        let mut child = Ui::new(
            self.ui.world,
            self.node,
            self.ui.records,
            self.ui.theme,
        );
        f(&mut child);
        self
    }

    /// Let this field travel rather than snap.
    ///
    /// Declares the lane and its curve; what it is *aimed* at is
    /// [`Fynix::aim`](crate::Fynix::aim), and until something aims it
    /// the base shows. The element is never written, so a target
    /// arriving mid flight carries on from where it had got to.
    pub fn transition<P>(
        &mut self,
        field: impl FnOnce(Cursor<Identity<E>>) -> Cursor<P>,
        transition: Transition<P::Target>,
    ) -> &mut Self
    where
        E: Send + Sync,
        P: FieldPath<Source = E>,
        P::Target: PartialEq + Clone + Send + Sync,
    {
        let cursor = field(Cursor::new());
        let accessor = cursor.accessor();

        // Where it starts is what the cascade left.
        let base = self
            .ui
            .records
            .elements
            .get::<E>(&self.node)
            .and_then(|element| (accessor.get)(element))
            .cloned();
        let Some(base) = base else {
            return self;
        };

        self.ui.records.lanes.insert(
            self.node,
            cursor.key(),
            Box::new(Travel::<H, E, P::Target> {
                accessor,
                hops: cursor.hops(),
                transition,
                shown: base.clone(),
                from: base.clone(),
                heading: base,
                target: None,
                elapsed: 0.0,
                host: PhantomData,
            }),
        );
        self
    }

    /// As [`transition`](Self::transition), but for
    /// [`build_fields`](crate::element::ElementVisual::build_fields):
    /// this node's element has not reached the kernel's own table yet
    /// (that only happens once `build` returns, and `build_fields`
    /// runs inside it), so there is nothing there yet to read a base
    /// from. `build_fields` already has `&self`, which is that same
    /// base - `base` is it, passed straight through instead of
    /// fetched.
    pub fn transition_from<P>(
        &mut self,
        field: impl FnOnce(Cursor<Identity<E>>) -> Cursor<P>,
        base: P::Target,
        transition: Transition<P::Target>,
    ) -> &mut Self
    where
        E: Send + Sync,
        P: FieldPath<Source = E>,
        P::Target: PartialEq + Clone + Send + Sync,
    {
        insert_lane::<H, E, P>(
            &mut self.ui.records.lanes,
            self.node,
            field,
            base,
            transition,
        );
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
        &mut self,
        field: impl FnOnce(Cursor<Identity<E>>) -> Cursor<P>,
        changed: impl ChangedFn<H>,
        value: impl Fn(&H::World, H::Node) -> P::Target
        + Send
        + Sync
        + 'static,
    ) -> &mut Self
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
