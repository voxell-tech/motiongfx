//! What a struct is as a piece of UI.
//!
//! `#[element]` writes it all: the [`ElementBase`] the cascade starts
//! from, and the [`Element`] that owns the `#[elem(child)]` children,
//! builds them first, and walks down to them when a change names one.
//! A field's own value writer and the optional structural hook are
//! named by `#[elem(patch = ...)]` and `#[element(build = ...)]`.

use crate::host::Host;
use crate::lenz::FieldId;
use crate::records::Records;
use crate::store::Store;

/// Marks a struct as an element. See
/// [`fynix_macros::element`](macro@fynix_macros::element).
pub use fynix_macros::element;

/// The value an element starts from, before a style and the call site.
///
/// Written by `#[element]`: the struct's `Default`, then each
/// `#[elem(default = ...)]` override, with the backend's theme in hand.
pub trait ElementBase<H: Host>: Sized {
    fn base(theme: &H::Theme) -> Self;
}

/// An element and the `#[elem(child)]` children beneath it.
///
/// Written by `#[element]`, for one backend.
pub trait Element<H: Host>: Fields {
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
    /// owner, then runs that field's `#[elem(patch = ...)]` writer.
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
