//! Typed, composable paths to the fields of a struct.
//!
//! A path is a *type*, not a value: it is zero-sized, composes
//! without allocating, and collapses to a pair of plain function
//! pointers once you are done building it. Access is fallible, so a
//! hop through an `Option` field is just another link rather than a
//! separate kind of path.
//!
//! Nothing here is specific to UI, or to any one kind of struct.
//!
//! ```ignore
//! let size = Card::path().header().badge().icon().size().accessor();
//! assert_eq!((size.get)(&card), Some(&12));
//! ```

use alloc::vec::Vec;
use core::any::TypeId;
use core::marker::PhantomData;

pub use fynix_mock_macros::Lenz;

/// What a path is called once the types are gone.
///
/// A path is a type, so its `TypeId` already identifies it, with no
/// string to build or compare. The wrapper keeps that an
/// implementation detail: an id can only come from a path, so nothing
/// can key a field on some unrelated type by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId(TypeId);

impl FieldId {
    /// The id naming `P`.
    ///
    /// Not public: an id is asked for by walking to the field, with
    /// [`FieldPath::id`] or
    /// [`Fields::field_id`](crate::element::Fields::field_id). Minting
    /// one any other way would name a field nothing points at.
    pub(crate) fn of<P: FieldPath + ?Sized>() -> Self {
        Self(TypeId::of::<P>())
    }
}

/// A path from `Source` to `Target`, composed at the type level.
///
/// A hop that cannot fail answers `Some` unconditionally, so callers
/// never need to know which links along the way were optional.
pub trait FieldPath: 'static {
    type Source: 'static;
    type Target: 'static;

    fn get(source: &Self::Source) -> Option<&Self::Target>;
    fn get_mut(
        source: &mut Self::Source,
    ) -> Option<&mut Self::Target>;

    /// Collapse this path into a pair of flat function pointers.
    ///
    /// `get`/`get_mut` are already monomorphized, so the pointers
    /// land on the field access directly once the `inline(always)`
    /// hops fold away. Implementors should not override this.
    fn erase() -> Accessor<Self::Source, Self::Target> {
        Accessor {
            get: Self::get,
            get_mut: Self::get_mut,
        }
    }

    /// Names this path once the types are gone, for keying a patch or
    /// a style entry.
    fn id() -> FieldId {
        FieldId::of::<Self>()
    }

    /// Appends the hops along this path, outermost first.
    ///
    /// One id per hop, so a walk that crosses into a nested struct
    /// says so: the owner recognises the first id and hands the rest
    /// to whoever owns the field. A single hop names a field of
    /// `Source` itself.
    fn ids(out: &mut Vec<FieldId>) {
        out.push(Self::id());
    }
}

/// The empty path, rooting every walk.
pub struct Identity<S>(PhantomData<fn() -> S>);

impl<S: 'static> FieldPath for Identity<S> {
    type Source = S;
    type Target = S;

    #[inline(always)]
    fn get(source: &S) -> Option<&S> {
        Some(source)
    }

    #[inline(always)]
    fn get_mut(source: &mut S) -> Option<&mut S> {
        Some(source)
    }

    /// The empty path has gone nowhere, so it names no hop.
    fn ids(_out: &mut Vec<FieldId>) {}
}

/// `A`, then `B`. The `B::Source == A::Target` bound rejects a
/// mismatched join at compile time, and the walk short circuits if
/// either link is absent.
pub struct Chain<A, B>(PhantomData<fn() -> (A, B)>);

impl<A, B> FieldPath for Chain<A, B>
where
    A: FieldPath,
    B: FieldPath<Source = A::Target>,
{
    type Source = A::Source;
    type Target = B::Target;

    #[inline(always)]
    fn get(source: &A::Source) -> Option<&B::Target> {
        A::get(source).and_then(B::get)
    }

    #[inline(always)]
    fn get_mut(source: &mut A::Source) -> Option<&mut B::Target> {
        A::get_mut(source).and_then(B::get_mut)
    }

    fn ids(out: &mut Vec<FieldId>) {
        A::ids(out);
        B::ids(out);
    }
}

/// Where the walk is standing. Carries the path so far as `P` and
/// nothing at all at runtime.
///
/// One cursor serves every struct: the methods come from the
/// `{Struct}Cursor` traits that `#[derive(Lenz)]` generates, so
/// there is no per-struct cursor type to name.
pub struct Cursor<P>(PhantomData<fn() -> P>);

impl<P> Cursor<P> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<P: FieldPath> Cursor<P> {
    /// Ends the walk, collapsing the path into flat function
    /// pointers.
    pub fn accessor(self) -> Accessor<P::Source, P::Target> {
        P::erase()
    }

    /// Ends the walk, naming the hops it took.
    ///
    /// The same walk gives both the write and where the write lands:
    /// [`accessor`](Self::accessor) reaches the value, this addresses
    /// it.
    pub fn ids(self) -> Vec<FieldId> {
        let mut out = Vec::new();
        P::ids(&mut out);
        out
    }
}

impl<P> Default for Cursor<P> {
    fn default() -> Self {
        Self::new()
    }
}

// A walk is worth reusing: the same one gives both the accessor and
// the ids. Nothing is stored, so copying it costs nothing.
impl<P> Clone for Cursor<P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P> Copy for Cursor<P> {}

/// What a finished path collapses to: two flat function pointers,
/// `Copy`, no allocation.
#[derive(Debug)]
pub struct Accessor<S, T> {
    pub get: fn(&S) -> Option<&T>,
    pub get_mut: fn(&mut S) -> Option<&mut T>,
}

impl<S, T> Clone for Accessor<S, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, T> Copy for Accessor<S, T> {}
