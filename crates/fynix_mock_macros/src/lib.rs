//! Derive macros for `fynix_mock`.

mod common;
mod element;
mod lenz;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

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
