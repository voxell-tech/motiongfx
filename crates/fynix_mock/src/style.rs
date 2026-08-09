//! What an element looks like before it is built, in three layers.
//!
//! An element starts at its own [`Default`], a [`Style`] writes over
//! that, and the call site writes over the style. A style is a
//! mutation rather than a set of values, so the order they run in is
//! the precedence, and nothing has to remember where a field's value
//! came from.
//!
//! [`StyledElem`] is what all three ways of asking for an element have
//! in common, so a builder takes one argument rather than three
//! overloads. [`elem!`](crate::elem) writes the right one.

use core::marker::PhantomData;

/// A look, as a mutation of an element that already has its defaults.
///
/// Nothing here names a backend: `apply` only writes fields, so one
/// style serves every [`Host`](crate::host::Host) the element is drawn
/// on.
pub trait Style {
    type Element: Default;

    /// A style meant to be applied more than once is implemented for
    /// `&Self`.
    fn apply(self, element: &mut Self::Element);
}

/// Anything that can produce a finished element.
pub trait StyledElem {
    type Element;

    /// Run the cascade, in order.
    fn create(self) -> Self::Element;
}

/// Default, then the style.
impl<S: Style> StyledElem for S {
    type Element = S::Element;

    fn create(self) -> S::Element {
        let mut elem = S::Element::default();
        self.apply(&mut elem);

        elem
    }
}

/// A style, and what the call site wants on top of it.
pub struct Inline<S, F>
where
    S: Style,
    F: FnOnce(&mut S::Element),
{
    pub style: S,
    pub inline: F,
}

impl<S, F> Inline<S, F>
where
    S: Style,
    F: FnOnce(&mut S::Element),
{
    /// The two halves, in the order they run.
    ///
    /// A closure passed here knows what it takes, because `S` says so.
    /// Written straight into the struct it would not, which is why
    /// [`elem!`](crate::elem) comes through this.
    pub fn new(style: S, inline: F) -> Self {
        Self { style, inline }
    }
}

/// Default, then the style, then the call site.
impl<S, F> StyledElem for Inline<S, F>
where
    S: Style,
    F: FnOnce(&mut S::Element),
{
    type Element = S::Element;

    fn create(self) -> S::Element {
        let mut elem = self.style.create();
        (self.inline)(&mut elem);

        elem
    }
}

/// An element that has already been built by hand.
///
/// The cascade is over before it starts: there is no default to write
/// over and no style to run, so this is only here to let a finished
/// element go where a [`StyledElem`] is asked for. It carries no
/// [`Default`] bound, because nothing ever calls for one.
pub struct Raw<E>(pub E);

impl<E> StyledElem for Raw<E> {
    type Element = E;

    fn create(self) -> E {
        self.0
    }
}

/// The style for a call site that only wants an apply of its own.
#[derive(Debug, Clone, Copy)]
pub struct NoStyle<E>(PhantomData<fn() -> E>);

impl<E> NoStyle<E> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<E> Default for NoStyle<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Default> Style for NoStyle<E> {
    type Element = E;

    fn apply(self, _: &mut Self::Element) {}
}
