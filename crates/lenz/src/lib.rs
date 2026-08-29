//! Typed, composable paths to the fields of a struct.
//!
//! A path is a *type*, not a value. It is zero-sized and collapses to
//! a pair of plain function pointers. A hop through an `Option` field
//! is just another link, not a separate kind of path.
//!
//! ```
//! use lenz::Lenz;
//! #[derive(Lenz)]
//! pub struct Card { pub header: Header }
//! #[derive(Lenz)]
//! pub struct Header { pub badge: Option<Badge> }
//! #[derive(Lenz)]
//! pub struct Badge { pub icon: Icon }
//! #[derive(Lenz)]
//! pub struct Icon { pub size: u32 }
//!
//! let card = Card {
//!     header: Header {
//!         badge: Some(Badge { icon: Icon { size: 12 } }),
//!     },
//! };
//! let size = Card::cursor().header().badge().icon().size().accessor();
//! assert_eq!(size.get(&card), Some(&12));
//! ```

#![no_std]

extern crate alloc;

// Lets `#[derive(Lenz)]` inside this crate's own tests resolve
// `::lenz` to itself.
extern crate self as lenz;

use alloc::vec::Vec;
use core::any::TypeId;
use core::marker::PhantomData;

pub use lenz_macros::Lenz;

/// What a path is called once the types are gone.
///
/// Wraps the path's own `TypeId`, so an id can only come from a real
/// path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId(TypeId);

impl FieldId {
    /// The id naming `P`.
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
    /// Do not override this.
    fn erase() -> Accessor<Self::Source, Self::Target> {
        Accessor {
            get: Self::get,
            get_mut: Self::get_mut,
        }
    }

    /// Names this path once the types are gone.
    fn id() -> FieldId {
        FieldId::of::<Self>()
    }

    /// Appends the hops along this path, outermost first.
    ///
    /// One id per hop. A single hop names a field of `Source` itself.
    fn hops(out: &mut Vec<FieldId>) {
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
    fn hops(_out: &mut Vec<FieldId>) {}
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

    fn hops(out: &mut Vec<FieldId>) {
        A::hops(out);
        B::hops(out);
    }
}

/// Where the walk is standing. Carries the path so far as `P`,
/// nothing at runtime.
///
/// One cursor serves every struct. Its methods come from the
/// `{Struct}Cursor` traits `#[derive(Lenz)]` generates.
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

    /// Ends the walk, naming the whole of it at once.
    ///
    /// One id for the entire path, distinct from any single
    /// [`hops`](Self::hops) id, so `top.text` and `bottom.text` key
    /// separately.
    pub fn key(self) -> FieldId {
        P::id()
    }

    /// Ends the walk, naming each hop it took. The route a patch
    /// follows.
    pub fn hops(self) -> Vec<FieldId> {
        let mut out = Vec::new();
        P::hops(&mut out);
        out
    }
}

impl<P> Default for Cursor<P> {
    fn default() -> Self {
        Self::new()
    }
}

// Free to copy: nothing is stored.
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
    get: fn(&S) -> Option<&T>,
    get_mut: fn(&mut S) -> Option<&mut T>,
}

impl<S, T> Accessor<S, T> {
    pub fn get<'s>(&self, source: &'s S) -> Option<&'s T> {
        (self.get)(source)
    }

    pub fn get_mut<'s>(
        &self,
        source: &'s mut S,
    ) -> Option<&'s mut T> {
        (self.get_mut)(source)
    }
}

impl<S, T> Clone for Accessor<S, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, T> Copy for Accessor<S, T> {}
