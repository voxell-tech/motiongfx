# Splitting `lenz` out of `fynix`

Plan, agreed but not built. The rest of `fynix` stays one crate: its
kernel (`element`, `ui`, `records`, `lanes`, `composer`, `Fynix`) is a
single dependency cycle and cannot be cut without turning `pub(crate)`
plumbing into `pub` API. `lenz` is the exception - `lenz.rs` names
nothing from the element model, and the dependency already runs
`fynix -> lenz`, never the other way.

If the real fynix repo (`~/develop/projects/rust/fynix`, which
`crates/fynix` models) diverges on crate structure, decide there first.

## Target layout

- `lenz` - the field-path types (`FieldPath`, `Chain`, `Identity`,
  `Cursor`, `Accessor`, `FieldId`). `no_std` + `alloc`. Re-exports
  `pub use lenz_macros::Lenz`. Gains a generic `#[lenz(ignore)]` field
  attribute; element's `#[elem(ignore)]` maps onto it.
- `lenz_macros` - `#[derive(Lenz, attributes(lenz))]`. Resolves its
  emitted path via `crate_name("lenz")`, with `#[lenz(crate = <path>)]`
  as a manual override (serde's `#[serde(crate = "...")]`).
- `macro_common` - the shared syn/quote helpers now in
  `fynix_macros/src/common.rs`: `generics`, `named_fields`,
  `option_inner`, `pascal_case`/`snake_case`, and
  `resolve_crate(name, fallback)` (today's `crate_path`, generalised
  off the hardcoded `"fynix"`). A plain library because two proc-macro
  crates cannot `use` each other's non-macro items. Vendoring the file
  into both macro crates is the accepted alternative.
- `fynix` - depends on `lenz`, `pub use lenz` for `fynix::lenz::...`
  typing (ergonomic only, not load-bearing), plus the `elem!`/`val!`
  macros.
- `fynix_macros` - `#[element]` and `#[derive(OverrideDefault)]`.
  Depends on `macro_common`, not on lenz's path codegen.
- `bevy_fynix` - unchanged, depends on `fynix`.

## `#[element]` as an attribute macro

`#[derive(Element)]` today re-runs the lenz codegen internally
(`lenz::expand_filtered`) alongside `OverrideDefault` and its own
dispatch enum. Replacing it with an attribute macro removes that reuse:
`#[element]` parses `#[elem(...)]` once, then emits ordinary derives and
hand-writes the rest.

```rust
#[element]
pub struct Button {
    pub label: String,
    #[elem(child)] pub icon: Icon,
    #[elem(ignore)] pub once: Handle,
    #[default(px(4))] pub radius: Val,
}
```

expands to

```rust
#[derive(::fynix::Lenz, ::fynix::OverrideDefault)]
#[lenz(crate = ::fynix::lenz)]
#[lenz(ignore(once))]
pub struct Button { pub label: String, pub icon: Icon, pub once: Handle, pub radius: Val }

#[derive(Clone, Copy)]
pub enum ButtonField { Label, Radius }        // child and ignore excluded
impl ::fynix::element::Fields for Button { /* ... */ }
impl<H> ::fynix::element::Element<H> for Button where /* ... */ { /* build/patch/despawn */ }
```

## Path resolution

A derive names the runtime crate in its output through
`proc-macro-crate`'s `crate_name`, which reads the *being-compiled*
crate's `Cargo.toml`. A crate that only depends on `fynix` (via its
`pub use lenz`) has no `lenz` entry, so `crate_name("lenz")` fails there
and `#[derive(Lenz)]` would not resolve.

`#[element]` sidesteps this. It runs in `fynix_macros`, resolves
`crate_name("fynix")` - which every element-using crate has as a direct
dependency - and emits `#[lenz(crate = ::fynix::lenz)]` on the struct.
`lenz_macros` honours that path and skips its own lookup. Downstream
crates need only `fynix`, and `lenz_macros` carries no knowledge of
`fynix` - the fynix-specific path is injected through the
`#[lenz(crate)]` seam.

A crate that wants `lenz` on its own writes `#[derive(Lenz)]` by hand,
depends on `lenz` directly, and gets the `crate_name("lenz")` default.

## Steps

Done:

- [x] `lenz` crate - the field-path types, moved out of
      `fynix/src/lenz.rs`. `fynix` re-exports it with `pub use ::lenz`,
      so `fynix::lenz::...` and `fynix::lenz::Lenz` still resolve.
- [x] `lenz_macros` crate - the `Lenz` derive, with `#[lenz(ignore)]`
      on a field and `#[lenz(crate = <path>)]` on the struct. Vendors
      the syn/quote helpers it needs rather than sharing a crate.
- [x] `#[derive(Element)]` unchanged - `fynix_macros` keeps its own
      copy of the lenz path codegen (`lenz::expand_filtered`) and
      emits paths to `::fynix::lenz`, which now resolves through the
      re-export.

Deferred:

- [ ] Rewrite `#[derive(Element)]` as the `#[element]` attribute
      macro, so it emits `#[derive(fynix::Lenz)] #[lenz(crate = ...)]`
      instead of carrying its own copy of the lenz codegen. Touches
      every `#[derive(Element)]` site (~40 files).
