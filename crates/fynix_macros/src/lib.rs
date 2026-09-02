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

/// Marks a struct as an element, built against one backend.
///
/// The struct is re-emitted with `#[derive(Lenz, OverrideDefault)]`,
/// then `Fields`, `ElementBase`, and `Element` are written for it. The
/// backend is `crate::FynixHost` unless `#[element(host = <path>)]` names
/// another.
///
/// - `#[element(build = <fn>)]` - a structural hook run once at build,
///   `fn(&Self, &mut Build<Host, Self>)`, that inserts components and
///   wires lanes and interactions. `#[element(build = Self::build)]`
///   points it at the element's own inherent `fn build`.
/// - `#[elem(child)]` - a field that is an element in its own right.
///   Built first, walked into one hop at a time.
/// - `#[elem(patch = <fn>)]` - an own field's value writer,
///   `fn(&mut Patch<Host>, &FieldTy)`, given a reference to that one
///   field. Run for every field at build and for the changed field on
///   patch.
/// - `#[elem(default = <expr>)]` - the value the field starts from,
///   with `theme` in scope. Layered over the `#[default(...)]` the
///   re-emitted struct still takes.
/// - `#[elem(ignore)]` - a field no path can name; read only by the
///   `build =` hook.
#[proc_macro_attribute]
pub fn element(args: TokenStream, input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match element::expand(args.into(), &ast) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
