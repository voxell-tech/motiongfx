# Overlay storage: a column per value type

Plan for the next round of `fynix` transition work: store running
overlays the way `motiongfx`'s `ActionTable` stores actions, so a
transition frame costs one contiguous column walk instead of chasing a
map full of boxed trait objects.

Builds on `overlay.rs` (commit `fb8c791`), where `Travel` became
`Tween<H, T>` and the writer became a `fn(&mut Patch<H>, &T)` pointer.

## Where we are

`Overlays<H>` is `HashMap<(H::Node, FieldId), Box<dyn Overlay<H>>>`.
Per overlay:

- a map entry: a 24 byte key and a 16 byte `Box` fat pointer,
- a heap `Tween<H, T>` behind that box, one allocation each, scattered.

The flush loop calls `overlay.advance(..)` through the vtable for every
entry. `Overlays::tween::<T>()` reaches the concrete `Tween` with a
`dyn Any` upcast and a `downcast_mut` behind a `debug_assert`.

`motiongfx` already solved the same shape. `ActionTable` is a
`TypeTable<ActionId>` (from `typarena`, the crate `fynix` already
depends on) with one column per value type. `Elements<H>` in
`records.rs` is a `TypeTable<H::Node>` for the same reason. The
per-type loops (`pipeline::bake` / `pipeline::sample`) are plain
monomorphised `fn`s reached through a registry of function pointers,
resolving each `TypeId` to its column once and looping.

`ActionTable` the struct does not transfer: its actions are baked once
into a `Segment<T>` and then sampled from a timeline queue driven by a
playhead. An overlay carries its own `elapsed` and advances every
frame. What transfers is the storage: `TypeTable` plus a fn-pointer
tick registry.

## The change

### 1. `Overlays<H>` over a `TypeTable`

```rust
pub struct Overlays<H: Host> {
    table: TypeTable<(H::Node, FieldId)>, // column per T: Tween<H, T>
    keys: HashSet<(H::Node, FieldId)>,    // the set to sweep on despawn
    ticks: Vec<(TypeId, TickFn<H>)>,      // one advance fn per T inserted
}

type TickFn<H> = fn(
    &mut TypeTable<(<H as Host>::Node, FieldId)>,
    f32,
    &mut <H as Host>::World,
    &<H as Host>::Theme,
);
```

`Tween<H, T>` loses its `Any` bound and its `node` is no longer stored
on it. `advance` becomes a plain inherent method.

```rust
fn tick<H: Host, T>(table, dt, world, theme)
where
    T: Clone + Send + Sync + 'static,
{
    for ((node, _), tween) in table.iter_mut::<Tween<H, T>>() {
        tween.advance(dt, world, *node, theme);
    }
}
```

`insert_overlay::<H, T>` is already generic over `T`. It records the
tick fn once per new `TypeId`, then `table.insert(key, tween)` (which
ensures the column) and `slots.insert(key, TypeId::of::<T>())`.

### 2. Everything that reached the concrete `Tween`

- `Fynix::aim` and `bind`'s rebase:
  `table.get_mut::<Tween<H, T>>(&(node, key))`, statically typed, no
  assert.
- `retain` on node despawn: walk `keys`, `table.remove_row` the dead
  ones, drop them from `keys`.
- flush loop: `overlays.advance(dt, world, theme)`, which calls each
  registered `tick` fn against `table`.
- `overlay_len` / `is_empty`: `keys.len()` / `keys.is_empty()`.

### 3. Gone

`trait Overlay<H>`, `Box<dyn Overlay<H>>`, the `Any` bound, the
`dyn Any` upcast and `downcast_mut` in `tween::<T>()`.

## Cost per overlay

|              | now                                | after                                     |
| ------------ | ---------------------------------- | ----------------------------------------- |
| map entry    | 24 B key, 16 B `Box` fat pointer   | 24 B key, 8 B `TypeId`                     |
| tween        | one heap allocation each, scattered | inline in a bulk-grown column, contiguous |
| advance loop | vtable call per entry              | one column walk per value type            |

## Upstream: `typarena`

`TypeTable` had `iter::<V>()` but no `iter_mut::<V>()`. Added in the
`voxell-tech/typarena` repo (`0.1.2`), mirroring `iter` with
`column_mut` and `IndexMap::iter_mut`. Nothing else was needed:
`get_mut`, `insert`, `remove_row` already exist.

Published as `0.1.2`; the workspace depends on it directly.

## Not in this cut

Drop-on-arrival and lazy seeding. A settled overlay stays allocated so
a hover `lit` transition can re-`aim` it. Removing it means `aim` has
to rebuild one from a stored recipe. Separate decision, layer on later
if a bench asks for it. This cut keeps semantics identical.

## Carried over from the old plan

- Delete `Element::patch`'s own-field walk. Both callers (`bind` and
  overlays) are off it now; keep the child-hop dispatch only if a path
  walk still has a user, keep `despawn`.
- Audit `records.elements` readers. Leave the table for
  `Fynix::element` and despawn, out of the reactive path.
- Bench a splitter drag and a playing timeline, before and after.

## Work items

- [x] `typarena`: `TypeTable::iter_mut::<V>()` (`7831ca6`, unpushed).
- [x] `Overlays<H>` over `TypeTable<(H::Node, FieldId)>` with a `keys`
      set and the `ticks` registry; `tick::<H, T>` fn.
- [x] `Tween<H, T>`: drop the `Any` bound, `advance` becomes inherent.
- [x] `insert_overlay` registers the tick fn per new `TypeId`.
- [x] `Fynix::aim`, `bind` rebase, `retain`, flush loop, `overlay_len`
      onto the new storage. `insert_overlay` / `tween()` signatures
      held, so `ui.rs` was untouched.
- [x] Delete `trait Overlay<H>` and the `dyn Any` path.
- [x] Publish `typarena` `0.1.2`; workspace depends on it directly, no
      patch.
- [ ] Delete `Element::patch`'s own-field walk.
- [ ] Audit `records.elements` readers.
- [ ] Bench a splitter drag and a playing timeline.
