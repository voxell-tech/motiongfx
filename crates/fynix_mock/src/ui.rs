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
use core::marker::PhantomData;

use crate::composer::Composer;
use crate::element::Element;
use crate::host::Host;
use crate::lanes::{Lanes, insert_lane, insert_travel};
use crate::lenz::{Cursor, FieldPath, Identity};
use crate::records::{
    Binding, BuildFn, ChangedFn, Elements, Records, Watcher,
};
use crate::store::Store;
use crate::style::StyledElem;
use crate::transition::Transition;

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

/// What [`patch_fields`](crate::element::ElementVisual::patch_fields)
/// writes through: the node a change already landed on, `world`, and
/// `theme` - the same three [`Draw`] gives `build_fields`, now for a
/// write a later change makes rather than the one a build did. No
/// [`Store`]/[`Lanes`] here: a patch writes an existing node's fields,
/// never wires a child or a lane the way a build can.
pub struct Patch<'a, H: Host> {
    pub world: &'a mut H::World,
    pub theme: &'a H::Theme,
    node: H::Node,
}

impl<'a, H: Host> Patch<'a, H> {
    /// Not for hand-written code: `#[derive(Element)]`'s own generated
    /// `patch` is what constructs this, once it has walked down to the
    /// element that owns the field a change named.
    #[doc(hidden)]
    pub fn new(
        world: &'a mut H::World,
        node: H::Node,
        theme: &'a H::Theme,
    ) -> Self {
        Self { world, theme, node }
    }

    /// This element's own node.
    pub fn id(&self) -> H::Node {
        self.node
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

        insert_travel(
            &mut self.ui.records.lanes,
            self.node,
            cursor.key(),
            accessor,
            cursor.hops(),
            base,
            transition,
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
                          store: &mut Store<H>,
                          theme: &H::Theme| {
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

            element.patch(world, node, &hops, store, theme);
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
