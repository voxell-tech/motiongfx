//! Derive macros for `fynix`.
//!
//! `#[derive(Lenz)]` for field paths lives in the `lenz` crate, which
//! `fynix` re-exports; `#[element]` derives what it and
//! `#[derive(OverrideDefault)]` would as part of its own output.

mod common;
mod element;
mod override_default;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Marks a struct as an element: the fields it draws itself become a
/// dispatch enum a `match` can check for exhaustiveness, and the
/// `#[elem(child)]` fields become children the tree builds first and
/// walks down to when a change names one.
///
/// The struct is re-emitted with `#[derive(Lenz, OverrideDefault)]`:
/// the dispatch names a field by the id `Lenz` gives it, and an
/// element's own fields almost always want `#[default(...)]`.
///
/// - `#[elem(child)]` - a field that is an element in its own right.
///   Absent from the enum; patches through its own id.
/// - `#[elem(ignore)]` - a field that only ever changes at build.
///   Absent from the enum and from the cursor, so nothing can name a
///   path to it.
#[proc_macro_attribute]
pub fn element(args: TokenStream, input: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "`#[element]` takes no arguments",
        )
        .into_compile_error()
        .into();
    }
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
///
/// `#[element]` emits this for its own struct - reach for it directly
/// only for one that is never an element at all.
#[proc_macro_derive(OverrideDefault, attributes(default))]
pub fn derive_override_default(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match override_default::expand(&ast) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
