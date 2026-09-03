//! The [`elem!`] macro: an element construction deferred until the
//! theme is in hand, as `FnOnce(&Theme) -> Element`.
//!
//! Three layers, in precedence order:
//! [`ElementBase::base`](crate::element::ElementBase::base) or
//! [`Seed`](crate::style::Seed) -> [`Style`](crate::style::Style) ->
//! the call site's own fields. [`Ui::elem`](crate::ui::Ui::elem) runs
//! the whole thing once, with the theme.

// For docs.
#[expect(unused_imports)]
use crate::elem;

/// The ways of asking for an element, as one deferred value.
///
/// Expands to a `move` closure `FnOnce(&<Host as Host>::Theme) ->
/// Element`. The arguments after the first are assignments to the
/// element, written without it; each converts through [`From`], and
/// `field = elem!(..)` starts that nested field from the same theme.
///
/// A field expression that reads the theme takes it from a local, not
/// through the `Ui`: the closure is `move`, so a bare `ui.theme` in
/// there captures `ui` itself.
///
/// ```
/// # #![allow(path_statements)]
/// # use fynix::elem;
/// # use fynix::style::Style;
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
/// #     fn delta(_: &()) -> f32 { 0.0 }
/// # }
/// # fn built<E>(_: impl FnOnce(&()) -> E) {}
/// #[derive(Default)]
/// struct Font { size: u32 }
///
/// #[derive(Default)]
/// struct Label { text: String, size: u32, font: Font }
///
/// impl ElementBase<Backend> for Label {
///     fn base(_theme: &()) -> Self { Self::default() }
/// }
///
/// struct Title;
/// impl Style for Title {
///     type Host = Backend;
///     type Element = Label;
///     fn apply(self, label: &mut Label, _theme: &()) { label.size = 10; }
/// }
///
/// elem!(!Title);                      // a style
/// elem!(!Title, size = 32u32);        // and writes over it
/// elem!(!Title, font.size = 32u32);   // as deep as they go
/// elem!(!Title, text = "Save");       // converting as they land
/// elem!(!Title, font = elem!(Font, size = 32u32)); // a value of its own
/// elem!(!Title, |l: &mut Label| { l.size += 2 }); // when it has to think
/// // No style here to carry the host, so `built` says it instead.
/// built(elem!(Label));                // no style, this element
/// built(elem!(Label, text = "Save")); // and its own fields
///
/// // Same ending either way; run order is precedence.
/// let label = elem!(!Title, text = "Save")(&());
///
/// assert_eq!(label.size, 10, "the style");
/// assert_eq!(label.text, "Save", "the call site");
/// ```
#[macro_export]
macro_rules! elem {
    // A style, marked `!`, with an apply of its own.
    (!$style:expr, |$($inline:tt)*) => {
        move |__theme: &_| {
            let mut __elem = $crate::style::styled($style, __theme);
            (|$($inline)*)(&mut __elem);
            __elem
        }
    };

    // A style, then the fields the call site writes over it.
    (!$style:expr $(, $($field:tt)*)?) => {
        move |__theme: &_| {
            let mut __elem = $crate::style::styled($style, __theme);
            $crate::fields!(__elem, __theme $(, $($field)*)?);
            __elem
        }
    };

    // No style: the common, unmarked case. The element from its
    // `Seed`, then the fields.
    ($elem:ty $(, $($field:tt)*)?) => {
        move |__theme: &_| {
            let mut __elem =
                <$elem as $crate::style::Seed<_>>::seed(__theme);
            $crate::fields!(__elem, __theme $(, $($field)*)?);
            __elem
        }
    };
}

/// Writes the call site's fields onto an element that already has its
/// defaults. `field = elem!(..)` resolves that nested build with
/// `theme`; anything else converts through [`From`].
#[macro_export]
#[doc(hidden)]
macro_rules! fields {
    (
        $target:ident, $theme:ident,
        $field:ident $(.$sub:ident)* = elem!($($nested:tt)*)
        $(, $($rest:tt)*)?
    ) => {
        $target.$field $(.$sub)* = ::core::convert::From::from(
            ($crate::elem!($($nested)*))($theme)
        );
        $crate::fields!($target, $theme $(, $($rest)*)?);
    };

    (
        $target:ident, $theme:ident,
        $field:ident $(.$sub:ident)* = $value:expr
        $(, $($rest:tt)*)?
    ) => {
        $target.$field $(.$sub)* =
            ::core::convert::From::from($value);
        $crate::fields!($target, $theme $(, $($rest)*)?);
    };

    ($target:ident, $theme:ident $(,)?) => {};
}
