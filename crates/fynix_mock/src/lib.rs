//! A mock of the fynix element model.

#![no_std]

extern crate alloc;

// Lets the derive emit `::fynix_mock::...` everywhere, including here.
extern crate self as fynix_mock;

mod elem;
pub mod element;
pub mod host;
pub mod kernel;
pub mod lenz;
pub mod store;
pub mod style;
pub mod ui;

/// Writes the `Default` an element starts from, before a style and then
/// a call site have had their say.
///
/// Nothing about it is UI: it applies to any struct or enum whose
/// fields want a default other than their own.
///
/// A field says what it starts as in one of these ways, none of which
/// name the field's type:
///
/// ```ignore
/// #[default(size: 24, weight: 400)] // its own default, overridden
/// #[default(0: 1, 1: 2)]            // the same, by index
/// #[default(_, 8, ..)]              // the same, as the pattern it
///                                   // looks like: `_` and `..` keep
///                                   // what the default had
/// #[default(::Bold)]                // a variant of the field's type
/// #[default(..)]                    // an `Option`, filled
/// ```
///
/// Anything else is the value, whole: `#[default(px(4))]`. Braces say
/// so outright, for a value that would otherwise read as one of the
/// above: `#[default({ ::core::f32::consts::PI })]`.
///
/// An enum starts in the variant marked `#[default]`, and that
/// variant's own fields take the attribute as any other field does.
pub use fynix_mock_macros::OverrideDefault;
