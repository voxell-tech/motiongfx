//! Derive macro for [`lenz`](https://docs.rs/lenz) field paths.

mod common;
mod paths;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Generates the field paths for a struct: a zero-sized `FieldPath`
/// marker per field, and a `Cursor` method that walks to it. Call
/// `accessor()` to end the walk.
///
/// A field marked `#[lenz(ignore)]` gets no marker and no cursor
/// method, so nothing can name a path to it.
///
/// `#[lenz(crate = ::path::to::lenz)]` on the struct overrides where
/// the generated code looks for the `lenz` crate - for a struct built
/// by a macro in another crate that only re-exports `lenz`.
#[proc_macro_derive(Lenz, attributes(lenz))]
pub fn derive_lenz(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match paths::expand(&ast) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
