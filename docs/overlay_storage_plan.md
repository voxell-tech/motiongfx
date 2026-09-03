# Tween storage: a column per value type

`fynix` transition work: store running tweens the way `motiongfx`'s
`ActionTable` stores actions, so a transition frame costs one
contiguous column walk instead of chasing a map full of boxed trait
objects.

Shipped in `7726341`, `8a3820c`, `7480bb8` on top of `b7a81c1`
(`Travel` -> `Tween<H, T>`, writer is a `fn(&mut Patch<H>, &T)`).

## What changed

### `TweenTable<H>` over a `TypeTable`

Was `HashMap<(H::Node, FieldId), Box<dyn Overlay<H>>>`: a heap
allocation and a vtable hop per transitioning field, plus a `dyn Any`
downcast behind a `debug_assert` to reach the concrete `Tween`.

Now (`crates/fynix/src/tween.rs`):

```rust
pub struct TweenTable<H: Host> {
    table: TypeTable<FieldKey<H>>,   // column per T: Tween<H, T>, inline
    keys: HashSet<FieldKey<H>>,      // the set to sweep on despawn
    ticks: Vec<(TypeId, TickFn<H>)>, // one advance fn per T inserted
}
```

`TypeTable` is `typarena`'s heterogeneous store, one column per value
type. `Elements<H>` (now `ElementTable<H>`) already used it. The
per-type advance is a monomorphised `fn` reached through the `ticks`
registry, resolving the column once and looping - the `motiongfx`
`Pipeline` pattern.

`ActionTable` the struct does not transfer: its actions bake into a
`Segment<T>` sampled from a timeline queue. A tween carries its own
`elapsed`. Only the storage transfers.

- `Tween<H, T>` lost its `Any` bound; `advance` is an inherent method;
  it no longer stores `node` (the key carries it).
- `Fynix::aim` and `bind`'s rebase: `TweenTable::running::<T>()`, a
  typed `TypeTable::get_mut`, no assert.
- flush loop: `tweens.advance(dt, world, theme)`.
- `retain`: walk `keys`, `table.remove_row` the dead ones.
- `trait Overlay<H>`, the `Box`, the `dyn Any` path: gone.

### One `FieldKey<H>` for the three field-keyed stores

`bindings`, `TweenTable`, and `Store::children` were each keyed by a
bare `(H::Node, FieldId)` tuple. `FieldKey<H>` names the pair once,
with `.node` / `.field`.

### Names

`Overlays` -> `TweenTable`, `overlay.rs` -> `tween.rs`,
`insert_overlay` -> `insert_tween`, `TweenTable::tween` -> `::running`
(no longer collides with `Tween`), `Elements<H>` -> `ElementTable<H>`,
`Fynix::overlay_len` -> `tween_len`, `Records::overlays` ->
`Records::tweens`. The word "overlay" is gone.

## Upstream: `typarena` 0.1.2

Added `TypeTable::iter_mut::<V>()`, mirroring `iter` with `column_mut`
and `IndexMap::iter_mut`. Published; the workspace depends on it
directly.

## Not in this cut

Drop-on-arrival and lazy seeding. A settled tween stays allocated so a
hover `lit` transition can re-`aim` it; removing it means `aim` rebuilds
one from a stored recipe. Layer on later if a bench asks for it.

## Follow-ups

- [ ] Delete `Element::patch`. Nothing outside the generated code calls
      it any more - `bind` and `transition` both go straight through the
      tag. It is still exercised directly by `crates/fynix/tests/`
      (`elements.rs`, `generics.rs`, `siblings.rs`), which would need
      rewriting onto the tag path or dropping. `build`'s `field_writes`
      and `despawn` stay; only `patch` (and its `own_patches` /
      `child_patches` codegen) goes. A focused change of its own.
- [x] Audit `records.elements` readers. Only `Fynix::element` (public,
      reads the latest), `despawn` (`remove_row`), the one-time base
      read in `ElementMut::transition`, and `bind`'s deliberate
      `*field = new` write-back. Nothing else walks it; no surprises.
- [ ] Bench a splitter drag and a playing timeline, before and after.
