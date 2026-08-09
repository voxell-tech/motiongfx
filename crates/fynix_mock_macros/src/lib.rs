//! Derive macros for `fynix_mock`.

mod common;
mod element;
mod lenz;
mod override_default;
mod style;

use proc_macro::TokenStream;
use syn::{DeriveInput, ItemFn, parse_macro_input};

/// Generates the field paths for a struct: a zero-sized
/// `FieldPath` marker per field, and a `Cursor` method that walks to
/// it. Call `accessor()` to end the walk.
#[proc_macro_derive(Lenz)]
pub fn derive_lenz(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match lenz::expand(&ast) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// Generates an enum naming the fields this element draws itself, so
/// dispatching on a field is a `match` the compiler checks for
/// exhaustiveness.
///
/// Fields marked `#[elem]` are left out: they are elements of their
/// own, and patch through their own id.
///
/// Needs `#[derive(Lenz)]` on the same struct: a variant reports the
/// id of the path marker that `Lenz` emits.
#[proc_macro_derive(Element, attributes(elem))]
pub fn derive_element(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match element::expand(&ast) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// Generates a `Default` impl where a field can start from something
/// other than its own default: `#[default(px(4))]` for a value, or
/// `#[default(size: 24)]` to keep the field's default and override
/// fields of it.
#[proc_macro_derive(OverrideDefault, attributes(default))]
pub fn derive_override_default(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match override_default::expand(&ast) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// Writes a `Style` from the function that applies it.
///
/// The struct is named after the function, carries every argument
/// after the first as a field, and the body writes to the first.
#[proc_macro_attribute]
pub fn style(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        let attr: proc_macro2::TokenStream = attr.into();
        return syn::Error::new_spanned(
            attr,
            "`#[style]` takes no arguments: the element is the first \
             one the function does",
        )
        .into_compile_error()
        .into();
    }

    let item = parse_macro_input!(item as ItemFn);
    match style::expand(&item) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
