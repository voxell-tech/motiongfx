//! What a struct is as a piece of UI, in two halves.
//!
//! [`ElementVisual`] is the half you write. It only sees the fields
//! this element draws itself. [`Element`] is the half
//! `#[element]` writes. It owns the `#[elem(child)]`
//! children, builds them first, and walks down to them when a change
//! names one.

use crate::host::Host;
use crate::lenz::FieldId;
use crate::records::Records;
use crate::store::Store;
use crate::ui::{Build, Patch};

/// Marks a struct as an element. See
/// [`fynix_macros::element`](macro@fynix_macros::element).
pub use fynix_macros::element;

/// An element and the `#[elem(child)]` children beneath it.
///
/// Written by `#[element]`, once per backend.
pub trait Element<H: Host>: ElementVisual<H> + Default {
    /// Build this element under `parent`, children and all.
    ///
    /// Each child's node is recorded in `records`, so
    /// [`patch`](Self::patch) can find it again.
    fn build(
        &self,
        world: &mut H::World,
        parent: H::Node,
        records: &mut Records<H>,
        theme: &H::Theme,
    ) -> H::Node;

    /// Apply a change named by a path, as
    /// [`Cursor::hops`](crate::lenz::Cursor::hops) reports it.
    ///
    /// Walks down one hop per element until it reaches the field's
    /// owner, then calls
    /// [`patch_fields`](ElementVisual::patch_fields).
    fn patch(
        &self,
        world: &mut H::World,
        node: H::Node,
        path: &[FieldId],
        store: &mut Store<H>,
        theme: &H::Theme,
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
/// Implemented by hand, once per struct and backend pair.
pub trait ElementVisual<H: Host>: Fields {
    /// Write this element's own fields onto `draw`'s own node.
    ///
    /// The node already exists, with its `#[elem(child)]` fields
    /// already built under it. Reach one with [`Build::child`].
    fn build_fields(&self, draw: &mut Build<H, Self>)
    where
        Self: Element<H> + Send + Sync;

    /// Push one changed field into visuals that already exist.
    ///
    /// A plain struct field is written whole. Elements are reached
    /// one hop at a time and never arrive here.
    fn patch_fields(&self, patch: &mut Patch<H>, field: Self::Field);
}

/// A struct whose own fields can be named one by one.
///
/// [`FieldId`] is opaque and can only be compared. Recovering the
/// enum instead gives a `match` the compiler checks against new
/// fields.
///
/// Fields marked `#[elem(child)]` are absent from the enum.
/// [`Element`] reaches those instead.
pub trait Fields: 'static {
    type Field: Copy + 'static;

    /// The field this id names, or `None` if it is not one this
    /// element draws.
    fn field(id: FieldId) -> Option<Self::Field>
    where
        Self: Sized;

    /// What `field` is called once the types are gone.
    ///
    /// Hangs off the struct, not the enum: a generic struct has one
    /// path marker per set of arguments, so the id cannot come from
    /// the variant alone.
    fn field_id(field: Self::Field) -> FieldId
    where
        Self: Sized;
}
