//! The `#[element(build = ...)]` hook's view of the world.

use core::marker::PhantomData;

use super::patch::Bindable;
use crate::element::Element;
use crate::host::Host;
use crate::lenz::{Cursor, FieldPath, Identity};
use crate::store::Store;
use crate::transition::{TransitionTable, insert_transition};
use crate::tween::Tween;

/// What a `#[element(build = ...)]` hook writes through: this
/// element's own node, `world`, `theme`, the child store, and the
/// transition table.
///
/// Not [`ElementMut`](crate::ui::ElementMut): a node running its own
/// build hook has not finished existing yet, so `bind`/`watch`/`with`
/// would not mean what they mean anywhere else.
pub struct Build<'a, H: Host, E: Element<H>> {
    pub world: &'a mut H::World,
    pub theme: &'a H::Theme,
    node: H::Node,
    transitions: &'a mut TransitionTable<H>,
    store: &'a mut Store<H>,
    element: PhantomData<fn() -> E>,
}

impl<'a, H: Host, E: Element<H>> Build<'a, H, E> {
    /// Not for hand-written code.
    #[doc(hidden)]
    pub fn new(
        world: &'a mut H::World,
        node: H::Node,
        transitions: &'a mut TransitionTable<H>,
        store: &'a mut Store<H>,
        theme: &'a H::Theme,
    ) -> Self {
        Self {
            world,
            theme,
            node,
            transitions,
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

    /// As
    /// [`ElementMut::transition_from`](crate::ui::ElementMut::transition_from),
    /// starting from `base`: this node has no entry in the kernel's
    /// table yet to read one from.
    pub fn transition_from<P>(
        &mut self,
        field: impl FnOnce(Cursor<Identity<E>>) -> Cursor<P>,
        base: P::Target,
        tween: Tween<P::Target>,
    ) -> &mut Self
    where
        E: Send + Sync,
        P: FieldPath<Source = E> + Bindable<H>,
        P::Target: Clone + Send + Sync,
    {
        let cursor = field(Cursor::new());
        let mut parents = cursor.hops();
        parents.pop();
        let Some(owner) = self.store.resolve(self.node, &parents)
        else {
            return self;
        };
        insert_transition(
            self.transitions,
            owner,
            cursor.key(),
            <P as Bindable<H>>::patch,
            base,
            tween,
        );
        self
    }
}
