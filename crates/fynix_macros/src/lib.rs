//! Derive macros for `fynix`.
//!
//! `#[derive(Lenz)]` for field paths lives in the `lenz` crate, which
//! `fynix` re-exports; `#[derive(Element)]` derives what it would as
//! part of its own output.

mod common;
mod element;
mod lenz;
mod override_default;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Generates an enum naming the fields this element draws itself, so
/// dispatching on a field is a `match` the compiler checks for
/// exhaustiveness.
///
/// Fields marked `#[elem(child)]` are left out: they are elements of
/// their own, and patch through their own id. Fields marked
/// `#[elem(ignore)]` are left out too, and out of the cursor `Lenz`
/// would otherwise give them: they only ever change at build, so
/// there is nothing for the field/patch system, or a path naming one
/// for it to reach, to find there.
///
/// Also emits what `#[derive(Lenz)]` and [`OverrideDefault`] would: an
/// element's own dispatch reports a field by the id `Lenz` gives it,
/// so the two have always gone together, and `#[default(...)]` is the
/// usual way an element's own fields differ from their type's default.
#[proc_macro_derive(Element, attributes(elem, default))]
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
///
/// [`Element`] already derives this for its own struct - reach for
/// this directly only for one that never derives `Element` at all.
#[proc_macro_derive(OverrideDefault, attributes(default))]
pub fn derive_override_default(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match override_default::expand(&ast) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
