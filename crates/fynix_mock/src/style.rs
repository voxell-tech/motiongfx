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

use crate::element::Element;
use crate::host::Host;
use crate::ui::ElementMut;

/// A look, as a mutation of an element that already has its defaults.
///
/// Nothing here names a backend: `apply` only writes fields, so one
/// style serves every [`Host`](crate::host::Host) the element is drawn
/// on. What it carries are the fields of the struct it is written on:
///
/// ```
/// use fynix_mock::style::{Style, StyledElem};
/// # use fynix_mock::host::Host;
/// # pub struct Backend;
/// # impl Host for Backend {
/// #     type Node = usize;
/// #     type World = ();
/// #     fn spawn(_: &mut (), _: usize) -> usize { 0 }
/// #     fn exists(_: &(), _: usize) -> bool { true }
/// #     fn children(_: &(), _: usize) -> Vec<usize> { Vec::new() }
/// #     fn despawn(_: &mut (), _: usize) {}
/// #     fn delta(_: &()) -> f32 { 0.0 }
/// # }
///
/// #[derive(Default)]
/// pub struct Label { size: u32, weight: u32 }
///
/// pub struct Heading { level: u32 }
///
/// impl Style for Heading {
///     type Host = Backend;
///     type Element = Label;
///
///     fn apply(self, label: &mut Label) {
///         label.size = 20 / self.level;
///
///         if self.level == 1 {
///             label.weight = 700;
///         }
///     }
/// }
///
/// let label = Heading { level: 1 }.create();
///
/// assert_eq!(label.size, 20);
/// assert_eq!(label.weight, 700);
/// ```
///
pub trait Style {
    /// The backend it moves on. A style that only writes fields moves
    /// on all of them and says so with a parameter of its own.
    type Host: Host;
    type Element: Default;

    /// A style meant to be applied more than once is implemented for
    /// `&Self`.
    ///
    /// Nothing, by default, for a style that only moves — see
    /// [`attach`](Self::attach).
    fn apply(self, element: &mut Self::Element)
    where
        Self: Sized,
    {
        let _ = element;
    }

    /// What this style hangs on the node once it exists: lanes, and
    /// what aims them. Nothing, by default.
    ///
    /// [`apply`](Self::apply) is the look and this is how it moves.
    /// Two methods, because one has an element and no node and the
    /// other a node and no element.
    fn attach(elem: ElementMut<Self::Host, Self::Element>)
    where
        Self: Sized,
        Self::Element: Element<Self::Host>,
    {
        let _ = elem;
    }
}

/// Anything that can produce a finished element.
pub trait StyledElem {
    type Host: Host;
    type Element;

    /// Run the cascade, in order.
    fn create(self) -> Self::Element;

    /// What this hangs on the node once it exists, which for a
    /// [`Style`] is [its own](Style::attach).
    fn attach(elem: ElementMut<Self::Host, Self::Element>)
    where
        Self: Sized,
        Self::Element: Element<Self::Host>,
    {
        let _ = elem;
    }
}

/// Default, then the style.
impl<S: Style> StyledElem for S {
    type Host = S::Host;
    type Element = S::Element;

    fn create(self) -> S::Element {
        let mut elem = S::Element::default();
        self.apply(&mut elem);

        elem
    }

    fn attach(elem: ElementMut<Self::Host, Self::Element>)
    where
        Self::Element: Element<Self::Host>,
    {
        <S as Style>::attach(elem);
    }
}

/// A style, and what the call site wants on top of it.
pub struct Inline<S, F>
where
    S: StyledElem,
    F: FnOnce(&mut S::Element),
{
    pub style: S,
    pub inline: F,
}

impl<S, F> Inline<S, F>
where
    S: StyledElem,
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
    S: StyledElem,
    F: FnOnce(&mut S::Element),
{
    type Host = S::Host;
    type Element = S::Element;

    fn create(self) -> S::Element {
        let mut elem = self.style.create();
        (self.inline)(&mut elem);

        elem
    }

    /// Whatever the style it wraps attaches: the call site writes
    /// values, not motion.
    fn attach(elem: ElementMut<Self::Host, Self::Element>)
    where
        Self::Element: Element<Self::Host>,
    {
        S::attach(elem);
    }
}

/// An element that has already been built by hand.
///
/// The cascade is over before it starts: there is no default to write
/// over and no style to run, so this is only here to let a finished
/// element go where a [`StyledElem`] is asked for. It carries no
/// [`Default`] bound, because nothing ever calls for one.
pub struct Raw<H, E>(E, PhantomData<fn() -> H>);

impl<H, E> Raw<H, E> {
    pub fn new(element: E) -> Self {
        Self(element, PhantomData)
    }
}

impl<H: Host, E> StyledElem for Raw<H, E> {
    type Host = H;
    type Element = E;

    fn create(self) -> E {
        self.0
    }
}

/// The style for a call site that only wants an apply of its own.
#[derive(Debug, Clone, Copy)]
pub struct NoStyle<H, E>(PhantomData<fn() -> (H, E)>);

impl<H, E> NoStyle<H, E> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<H, E> Default for NoStyle<H, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<H: Host, E: Default> Style for NoStyle<H, E> {
    type Host = H;
    type Element = E;

    fn apply(self, _: &mut Self::Element) {}
}
