//! What an element looks like before it is built, in four layers.
//!
//! An element starts at its own [`ElementBase::base`], a [`Style`]
//! writes over that with [`Style::apply`], the call site writes over
//! the style, then the style gets the last word with
//! [`Style::finish`] - late enough to reach a child the call site
//! just built, which `apply` runs too early to see.
//! [`elem!`](crate::elem!) bundles all four into an
//! `FnOnce(&Theme) -> Element` that [`Ui::elem`](crate::ui::Ui::elem)
//! runs once it has the theme.
//!
//! A style only writes fields. It never sees a node, so it cannot
//! wire an observer or a transition.

use crate::element::ElementBase;
use crate::host::Host;

/// A look, as a mutation of an element that already has its defaults.
///
/// One style serves every [`Host`] the element is drawn on. What it
/// carries are the fields of the struct it is written on:
///
/// ```
/// use fynix::style::Style;
/// use fynix::elem;
/// # use fynix::host::Host;
/// # use fynix::element::ElementBase;
/// # pub struct Backend;
/// # impl Host for Backend {
/// #     type Node = usize;
/// #     type World = ();
/// #     type Theme = ();
/// #     fn spawn(_: &mut (), _: usize) -> usize { 0 }
/// #     fn exists(_: &(), _: usize) -> bool { true }
/// #     fn children(_: &(), _: usize) -> Vec<usize> { Vec::new() }
/// #     fn despawn(_: &mut (), _: usize) {}
/// #     fn delta(_: &()) -> core::time::Duration { core::time::Duration::ZERO }
/// # }
///
/// #[derive(Default)]
/// pub struct Label { size: u32, weight: u32 }
///
/// impl ElementBase<Backend> for Label {
///     fn base(_theme: &()) -> Self { Self::default() }
/// }
///
/// pub struct Heading { level: u32 }
///
/// impl Style for Heading {
///     type Host = Backend;
///     type Element = Label;
///
///     fn apply(&self, label: &mut Label, _theme: &()) {
///         label.size = 20 / self.level;
///
///         if self.level == 1 {
///             label.weight = 700;
///         }
///     }
/// }
///
/// let label = elem!(!Heading { level: 1 })(&());
///
/// assert_eq!(label.size, 20);
/// assert_eq!(label.weight, 700);
/// ```
///
pub trait Style {
    /// The backend it moves on. A style that only writes fields moves
    /// on all of them and says so with a parameter of its own.
    type Host: Host;
    type Element: ElementBase<Self::Host>;

    /// Before the call site's own fields land.
    ///
    /// `theme` is [`Host::Theme`] - read from, never written: a style
    /// decides its own look from it, but has no node yet to leave
    /// anything reactive against it.
    fn apply(
        &self,
        element: &mut Self::Element,
        theme: &<Self::Host as Host>::Theme,
    ) {
        let _ = (element, theme);
    }

    /// After them - late enough to reach a child the call site just
    /// built, which `apply` runs too early to see. Most styles never
    /// need this; the default does nothing.
    fn finish(
        &self,
        element: &mut Self::Element,
        theme: &<Self::Host as Host>::Theme,
    ) {
        let _ = (element, theme);
    }
}

/// [`ElementBase::base`] then [`Style::apply`], the first two layers
/// of an [`elem!`](crate::elem!) with a `!style`. The style is
/// borrowed, not consumed: [`Style::finish`] still needs it once the
/// call site's fields have landed.
#[doc(hidden)]
pub fn styled<S: Style>(
    style: &S,
    theme: &<S::Host as Host>::Theme,
) -> S::Element {
    let mut element =
        <S::Element as ElementBase<S::Host>>::base(theme);
    style.apply(&mut element, theme);

    element
}

/// A field's starting value in an [`elem!`](crate::elem!): an
/// element's [`ElementBase::base`], or any other type's [`Default`].
pub trait Seed<Th> {
    fn seed(theme: &Th) -> Self;
}

impl<Th, T: Default> Seed<Th> for T {
    fn seed(_theme: &Th) -> Self {
        T::default()
    }
}
