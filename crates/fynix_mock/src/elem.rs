//! The [`elem!`] macro to create elements for [`Ui::elem()`].
//! The aim is to enforce the styling order of
//! [`Default`] -> [`Style`] -> [`Inline`].
//!
//! [`Ui::elem()`]: crate::ui::Ui::elem()
//! [`Style`]: crate::style::Style
//! [`Inline`]: crate::style::Inline

/// The ways of asking for an element, as one value.
///
/// The braces are assignments to the element, written without it, and
/// each one converts through [`From`]. What comes out goes to
/// [`Ui::elem`](crate::ui::Ui::elem).
///
/// ```
/// # #![allow(path_statements)]
/// # use fynix_mock::elem;
/// # use fynix_mock::style::{Style, StyledElem};
/// #[derive(Default)]
/// struct Font { size: u32 }
///
/// #[derive(Default)]
/// struct Label { text: String, size: u32, font: Font }
///
/// struct Title;
/// impl Style for Title {
///     type Element = Label;
///     fn apply(self, label: &mut Label) { label.size = 10; }
/// }
///
/// elem!(Title);                        // a style
/// elem!(Title, { size = 32u32 });      // and writes over it
/// elem!(Title, { font.size = 32u32 }); // as deep as they go
/// elem!(Title, { text = "Save" });     // converting as they land
/// elem!(Title, |l: &mut Label| { l.size += 2 });    // when it has to think
/// elem!(!Label);                       // no style, this element
/// elem!(!Label { text = "Save" });     // and its own fields
///
/// // Whichever it was, it ends the same way, and the order it ran in
/// // is the precedence.
/// let label = elem!(Title, { text = "Save" }).create();
///
/// assert_eq!(label.size, 10, "the style");
/// assert_eq!(label.text, "Save", "the call site");
/// ```
#[macro_export]
macro_rules! elem {
    // No style: `!` for the element itself. The same cascade, with
    // nothing in the middle, so what is left out is the element's own
    // default.
    (!$elem:ty { $($field:ident $(.$sub:ident)* = $value:expr);* $(;)? }) => {
        $crate::elem!(
            $crate::style::NoStyle::<$elem>::new(),
            { $($field $(.$sub)* = $value);* }
        )
    };

    (!$elem:ty) => {
        $crate::style::NoStyle::<$elem>::new()
    };

    // A style, then the fields the call site writes over it.
    ($style:expr, { $($field:ident $(.$sub:ident)* = $value:expr);* $(;)? }) => {
        $crate::style::Inline::new($style, |__elem: &mut _| {
            $(
                __elem.$field $(.$sub)* =
                    ::core::convert::From::from($value);
            )*
        })
    };

    // An apply that is not a list of fields. Leading `|`, so it is a
    // closure and not a style, and the element is what it takes.
    (|$($inline:tt)*) => {
        $crate::style::Inline::new(
            $crate::style::NoStyle::new(),
            |$($inline)*,
        )
    };

    ($style:expr, |$($inline:tt)*) => {
        $crate::style::Inline::new($style, |$($inline)*)
    };

    // A style alone, which is already a `StyledElem`.
    ($style:expr) => {
        $style
    };
}
