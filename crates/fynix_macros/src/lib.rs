//! Derive macros for `fynix`.
//!
//! `#[derive(Lenz)]` for field paths lives in the `lenz` crate,
//! re-exported by `fynix`; `#[element]` emits what it and its own
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
/// - `#[elem(patch = <tag>)]` - a type implementing
///   `FieldPatch<Host, Target = FieldTy>`. Its `patch` writes the
///   field, at build and on change.
/// - `#[elem(default = <expr>)]` - the value the field starts from in
///   [`ElementBase::base`], with `theme` in scope. `default = ::<expr>`
///   resolves against the field's own type: `::NONE` is `<Color>::NONE`
///   on a `Color` field. A field with none starts from its own
///   [`Default`], a `#[elem(child)]` one from its own `base`.
/// - `#[elem(ignore)]` - a field no path can name, read only by the
///   `build =` hook.
///
/// A `Default` impl is written too, deferring to `base` with
/// `<Host::Theme>::default()`, for `val!` and nested construction.
#[proc_macro_attribute]
pub fn element(args: TokenStream, input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match element::expand(args.into(), &ast) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
