//! What a struct is as a piece of UI.
//!
//! `#[element]` writes it all: the [`ElementBase`] the cascade starts
//! from, and the [`Element`] that owns the `#[elem(child)]` children,
//! builds them first, and walks down to them when a change names one.
//! A field's own value writer and the optional structural hook are
//! named by `#[elem(patch = ...)]` and `#[element(build = ...)]`.

use crate::anim::Access;
use crate::host::Host;
use crate::lenz::FieldId;
use crate::records::Records;
use crate::store::Store;

/// Marks a struct as an element. See
/// [`fynix_macros::element`](macro@fynix_macros::element).
pub use fynix_macros::element;

/// The value an element starts from, before a style and the call site.
///
/// `#[element]` writes it field by field: an `#[elem(default = ...)]`
/// value with the theme in hand, or the field's own `Default`.
pub trait ElementBase<H: Host>: Sized {
    fn base(theme: &H::Theme) -> Self;
}

/// An element and the `#[elem(child)]` children beneath it.
///
/// Written by `#[element]`, for one backend. `'static` so a built
/// element can go into a type-erased table.
pub trait Element<H: Host>: 'static {
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

/// Builds a `#[elem(child)]` field, if present, then records and
/// mounts it; see [`Records::mount_child`].
#[doc(hidden)]
pub fn build_child<H, E>(
    elem: Option<&E>,
    id: FieldId,
    world: &mut H::World,
    node: H::Node,
    records: &mut Records<H>,
    theme: &H::Theme,
    read: Access<H, E>,
) where
    H: Host,
    E: Element<H> + Send + Sync + 'static,
{
    let Some(elem) = elem else {
        return;
    };
    let child = elem.build(world, node, records, theme);
    records.store_mut().insert(node, id, child);
    records.mount_child(child, node, read);
}

/// Patches a `#[elem(child)]` field if `head` names it, returning
/// whether it did.
#[doc(hidden)]
#[expect(clippy::too_many_arguments)]
pub fn patch_child<H, E>(
    elem: Option<&E>,
    id: FieldId,
    node: H::Node,
    head: FieldId,
    rest: &[FieldId],
    world: &mut H::World,
    store: &mut Store<H>,
    theme: &H::Theme,
) -> bool
where
    H: Host,
    E: Element<H>,
{
    if head != id {
        return false;
    }
    if let (Some(elem), Some(child)) = (elem, store.get(node, head)) {
        elem.patch(world, child, rest, store, theme);
    }
    true
}

/// Despawns a `#[elem(child)]` field's subtree, if present and built.
#[doc(hidden)]
pub fn despawn_child<H, E>(
    elem: Option<&E>,
    id: FieldId,
    node: H::Node,
    world: &mut H::World,
    store: &mut Store<H>,
) where
    H: Host,
    E: Element<H>,
{
    if let (Some(elem), Some(child)) = (elem, store.take(node, id)) {
        elem.despawn(world, child, store);
    }
}
