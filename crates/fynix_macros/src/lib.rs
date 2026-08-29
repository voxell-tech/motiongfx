//! Derive macros for `fynix`.
//!
//! `#[derive(Lenz)]` for field paths lives in the `lenz` crate and
//! `#[derive(OverrideDefault)]` in the `override_default` crate, both
//! re-exported by `fynix`; `#[element]` emits what they and its own
//! dispatch would.

mod common;
mod element;

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
