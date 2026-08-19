//! Where a transitioning field lives between the base an element
//! carries and whatever it is aimed at.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::any::Any;
use core::marker::PhantomData;

use hashbrown::HashMap;

use crate::element::Element;
use crate::host::Host;
use crate::lenz::{Accessor, Cursor, FieldId, FieldPath, Identity};
use crate::records::Elements;
use crate::store::Store;
use crate::transition::Transition;

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
        theme: &H::Theme,
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
        theme: &H::Theme,
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

        element.patch(world, node, &self.hops, store, theme);

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
/// [`Draw`](crate::ui::Draw) holds one directly, the same way it holds
/// a [`Store`], without either leaking what a lane actually is.
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

/// What every lane starts from, whichever of `field`'s two callers
/// already has an [`Accessor`] in hand and which only has a path to
/// get one from.
pub(crate) fn insert_travel<H, E, T>(
    lanes: &mut Lanes<H>,
    node: H::Node,
    key: FieldId,
    accessor: Accessor<E, T>,
    hops: Vec<FieldId>,
    base: T,
    transition: Transition<T>,
) where
    H: Host,
    E: Element<H> + Send + Sync,
    T: PartialEq + Clone + Send + Sync + 'static,
{
    lanes.insert(
        node,
        key,
        Box::new(Travel {
            accessor,
            hops,
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

/// The shared insides of
/// [`ElementMut::transition_from`](crate::ui::ElementMut::transition_from)
/// and [`Draw::transition_from`](crate::ui::Draw::transition_from) -
/// only where the lane table comes from differs between them.
pub(crate) fn insert_lane<H, E, P>(
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

    insert_travel(
        lanes,
        node,
        cursor.key(),
        accessor,
        cursor.hops(),
        base,
        transition,
    );
}
