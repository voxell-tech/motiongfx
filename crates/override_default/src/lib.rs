#![doc = include_str!("../README.md")]

mod expand;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Generates a `Default` impl where a field can start from something
/// other than its own default: `#[default(px(4))]` for a value, or
/// `#[default(size: 24)]` to keep the field's default and override
/// fields of it.
///
/// - `#[default(px(4))]` - this value, whole.
/// - `#[default(::Bold)]` - a variant of the field's own type.
/// - `#[default(size: 24, weight: 400)]` - the field's own default,
///   with these of its fields overridden (`0: 1, 1: 2` for a tuple).
/// - `#[default(_, 8, ..)]` - the same, written as the pattern it
///   looks like; `_` and `..` keep what the default had.
/// - `#[default(..)]` - an `Option` field holding its inner default.
///
/// An enum starts in the variant marked `#[default]`, and that
/// variant's own fields take the attribute as any other field does.
#[proc_macro_derive(OverrideDefault, attributes(default))]
pub fn derive_override_default(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match expand::expand(&ast) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
