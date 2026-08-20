//! The [`elem!`] macro to create elements for [`Ui::elem()`].
//! The aim is to enforce the styling order of
//! [`Default`] -> [`Style`] -> [`Inline`].
//!
//! [`Ui::elem()`]: crate::ui::Ui::elem()
//! [`Style`]: crate::style::Style
//! [`Inline`]: crate::style::Inline

// For docs.
#[expect(unused_imports)]
use crate::elem;

/// The ways of asking for an element, as one value.
///
/// The arguments after the first are assignments to the element,
/// written without it, and each one converts through [`From`]. What
/// comes out goes to [`Ui::elem`](crate::ui::Ui::elem).
///
/// ```
/// # #![allow(path_statements)]
/// # use fynix_mock::{elem, val};
/// # use fynix_mock::style::{Style, StyledElem};
/// # use fynix_mock::host::Host;
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
/// # fn built<S: StyledElem<Host = Backend>>(_: S) {}
/// #[derive(Default)]
/// struct Font { size: u32 }
///
/// #[derive(Default)]
/// struct Label { text: String, size: u32, font: Font }
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
/// elem!(!Title, font = val!(Font, size = 32u32)); // a value of its own
/// elem!(!Title, |l: &mut Label| { l.size += 2 }); // when it has to think
/// // The host is the style's, and there is no style here, so these
/// // two are the one case that has to be told: anywhere they are
/// // built the host is already known.
/// built(elem!(Label));                // no style, this element
/// built(elem!(Label, text = "Save")); // and its own fields
///
/// // Whichever it was, it ends the same way, and the order it ran in
/// // is the precedence.
/// let label = elem!(!Title, text = "Save").create(&());
///
/// assert_eq!(label.size, 10, "the style");
/// assert_eq!(label.text, "Save", "the call site");
/// ```
#[macro_export]
macro_rules! elem {
    // A style, marked `!`, with an apply of its own.
    (!$style:expr, |$($inline:tt)*) => {
        $crate::style::Inline::new($style, |$($inline)*)
    };

    // A style, then the fields the call site writes over it.
    (!$style:expr $(, $($field:tt)*)?) => {
        $crate::style::Inline::new($style, |__elem: &mut _| {
            $crate::fields!(__elem $(, $($field)*)?);
        })
    };

    // An apply that is not a list of fields. Leading `|`, so it is a
    // closure and not an element, and the element is what it takes.
    (|$($inline:tt)*) => {
        $crate::style::Inline::new(
            $crate::style::NoStyle::new(),
            |$($inline)*,
        )
    };

    // No style: the element itself, which is the common case and so
    // the unmarked one. The same cascade with nothing in the middle,
    // so what is left out is the element's own default.
    ($elem:ty $(, $($field:tt)*)?) => {
        $crate::elem!(
            !$crate::style::NoStyle::<_, $elem>::new()
            $(, $($field)*)?
        )
    };
}

/// A value that starts from its [`Default`] and takes the fields
/// named, for what an element holds rather than what it is.
///
/// The same assignments [`elem!`](crate::elem!) takes, so a value
/// nested in one reads like the element around it:
///
/// ```
/// # use fynix_mock::val;
/// #[derive(Default)]
/// struct Font { size: u32, weight: u32 }
///
/// let font = val!(Font, size = 24u32);
///
/// assert_eq!((font.size, font.weight), (24, 0));
/// ```
#[macro_export]
macro_rules! val {
    // A style, marked `!` as in [`elem!`], run down to the element it
    // makes. What it hangs on a node is lost: a nested value has no
    // node of its own until its owner builds it - and no `Ui` in
    // scope to read a real theme from either, so `apply` gets its
    // host's default one instead.
    (!$style:expr $(, $($field:tt)*)?) => {
        $crate::style::StyledElem::create(
            $crate::elem!(!$style $(, $($field)*)?),
            &::core::default::Default::default(),
        )
    };

    ($path:path $(, $($field:tt)*)?) => {{
        let mut __value =
            <$path as ::core::default::Default>::default();
        $crate::fields!(__value $(, $($field)*)?);
        __value
    }};
}

/// Writes fields onto a value that already exists, which is the half
/// [`elem!`] and [`val!`] have in common.
#[macro_export]
macro_rules! fields {
    (
        $target:ident,
        $field:ident $(.$sub:ident)* = $value:expr
        $(, $($rest:tt)*)?
    ) => {
        $target.$field $(.$sub)* =
            ::core::convert::From::from($value);
        $crate::fields!($target $(, $($rest)*)?);
    };

    ($target:ident $(,)?) => {};
}
