//! What a struct is as a piece of UI, in two halves.
//!
//! [`ElementVisual`] is the half you write, and it only ever sees the
//! fields this element draws itself. [`Element`] is the half
//! `#[derive(Element)]` writes: it owns the `#[elem]` children, builds
//! them before the element's own fields exist, and walks down to them
//! when a change names one.
//!
//! So an impl never mentions a child, and a child's visuals are only
//! ever described in one place, its own impl.

use crate::host::Host;
use crate::lenz::FieldId;
use crate::store::Store;
use crate::ui::{Records, Ui};

// Same name as the trait, in the macro namespace, the way `Default`
// and `Clone` do it.
pub use fynix_mock_macros::Element;

/// An element and the `#[elem]` children beneath it.
///
/// Written by `#[derive(Element)]`, once for every backend at a time:
/// the owner never names a child's backend, and never says twice how
/// a child is drawn.
pub trait Element<H: Host>: ElementVisual<H> + Default {
    /// Build this element under `parent`, children and all.
    ///
    /// Every child's node is recorded in `records`, which is what lets
    /// [`patch`](Self::patch) find it again - and what
    /// [`build_fields`](ElementVisual::build_fields) gets handed as a
    /// [`Ui`] of its own, rooted on the node this returns.
    fn build(
        &self,
        world: &mut H::World,
        parent: H::Node,
        records: &mut Records<H>,
    ) -> H::Node;

    /// Apply a change named by a path, as
    /// [`Cursor::hops`](crate::lenz::Cursor::hops) reports it.
    ///
    /// A path naming a child is walked down, one hop per element,
    /// until it reaches the one that owns it. Anything else is this
    /// element's own field, and goes to
    /// [`patch_fields`](ElementVisual::patch_fields).
    fn patch(
        &self,
        world: &mut H::World,
        node: H::Node,
        path: &[FieldId],
        store: &mut Store<H>,
    );

    /// Destroy this element, its children, and what the store holds
    /// for them.
    fn despawn(
        &self,
        world: &mut H::World,
        node: H::Node,
        store: &mut Store<H>,
    );
}

/// How an element draws its own fields on one backend.
///
/// Implemented by hand, once per (struct, backend) pair. This is the
/// only place that knows both what the data means and how the backend
/// draws it, and it is not concerned with children.
pub trait ElementVisual<H: Host>: Fields {
    /// Write this element's own fields onto `ui.parent()`.
    ///
    /// The node already exists, and its `#[elem]` fields are already
    /// built under it - reach one with [`Ui::child`]. `ui` is real:
    /// nothing stops this from building further children of its own
    /// with [`Ui::elem`]/[`Ui::compose`], the way any other build
    /// would.
    fn build_fields(&self, ui: &mut Ui<'_, H>);

    /// Push one changed field into visuals that already exist.
    ///
    /// A field naming a plain struct is written whole. Only elements
    /// are reached one hop at a time, and those never arrive here.
    fn patch_fields(
        &self,
        world: &mut H::World,
        node: H::Node,
        field: Self::Field,
    );
}

/// A struct whose own fields can be named one by one.
///
/// [`FieldId`] is opaque, so code that receives one can only compare
/// it. Recovering the enum instead gives a `match`, and a `match` the
/// compiler checks: gain a field, and every place that dispatches on
/// one stops compiling until it says what the new field means.
///
/// Fields marked `#[elem]` are absent from the enum. They are
/// elements themselves, and [`Element`] reaches them.
pub trait Fields: 'static {
    type Field: Copy + 'static;

    /// The field this id names, or `None` if it is not one this
    /// element draws.
    fn field(id: FieldId) -> Option<Self::Field>
    where
        Self: Sized;

    /// What `field` is called once the types are gone.
    ///
    /// It hangs off the struct rather than the enum because a generic
    /// struct has one path marker per set of arguments, so the id
    /// cannot be read from the variant alone.
    fn field_id(field: Self::Field) -> FieldId
    where
        Self: Sized;
}
