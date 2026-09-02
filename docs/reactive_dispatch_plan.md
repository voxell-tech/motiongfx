# Reactive dispatch: bindings and lanes without walking the tree

Plan for the next round of `fynix` kernel work: cut the per-fire cost of
a binding, and give transitions a backend that writes one component and
never touches the retained element. Builds on the `#[element]` redesign
(`nixon/moxie/fynix-patch`), where every field's writer became
`fn(&mut Patch<H>, &FieldTy)` - the value is an argument now, not
something read back off `&self`.

## Where we are

A binding stores a closure that runs on every `changed` fire
(`ui.rs`, `ElementMut::bind`):

1. `value(WorldNodeRef::new(world, node))` - read the new value.
2. `elements.get_mut::<E>(&node)` - typed lookup into the retained
   element table.
3. `accessor.get_mut(element)` - walk the getter chain to the field,
   hopping through `Option`s.
4. `*field = new` - write it into the retained struct.
5. `element.patch(world, node, &hops, store, theme)` - recurse one
   `Element::patch` frame per child hop, then linear-scan the owner's
   own-field `if *head == id` arms, then call that field's
   `#[elem(patch = ...)]` writer with `&self.field`.

A lane (`lanes.rs`, `Travel::advance`) does the same walk, plus a
smuggle: it can't pass the animated value as an argument through the old
path, so it does

```rust
let base = core::mem::replace(field, self.shown.clone());
element.patch(world, node, &self.hops, store, theme);
if let Some(field) = self.accessor.get_mut(element) { *field = base; }
```

writing `shown` into the struct, patching, then putting the base back.

Every `changed` fire and every transition frame pays for steps 2, 3 and
5. For a field bound three elements deep that is three virtual
`Element::patch` calls and three re-scans, per frame, for a splitter
drag or a playhead in motion.

`Element::patch` has exactly two callers - `ui.rs` (bindings) and
`lanes.rs` (lanes). Nothing else walks it.

## The change

### 1. Resolve a binding's target once

The `hops` path is fixed for a binding's whole life. Resolve it at bind
time instead of per fire:

- Walk `store` (`store.child(node, id)` per hop) to the **owner node**.
- Capture the terminal field's writer, `fn(&mut Patch<H>, &T)`.

Store `(owner_node, writer)` on the `Binding`. The fire path becomes:

```rust
let new = value(WorldNodeRef::new(world, node));
if let Some(el) = elements.get_mut::<E>(&node) {   // keep - see caveats
    accessor.get_mut(el).map(|f| *f = new.clone());
}
let mut patch = Patch::new(world, owner_node, theme);
(writer)(&mut patch, &new);
```

No recursion, no scan.

**Getting the writer out of the cursor.** Since the `lenz` split the
cursor and `FieldPath` are host-agnostic, so the `Patch<H>`-typed
writer can't hang off them. Add a `fynix`-side trait the `#[element]`
macro implements per field, on that field's terminal cursor segment:

```rust
pub trait OwnField<H: Host>: FieldPath {
    fn write(patch: &mut Patch<H>, value: &Self::Target);
}
```

`bind` bounds `P: OwnField<H>` and calls `P::write`. Composed hops
expose the terminal field's impl, so `b.icon().color()` yields `Icon`'s
`color` writer and the path to `Icon`'s node.

### 2. A transition backend that never reads the element

With an argument-taking writer the smuggle disappears. A lane holds its
own state and pushes straight to the component:

```rust
struct Travel<H: Host, T> {
    node: H::Node,
    write: fn(&mut Patch<H>, &T),
    transition: Transition<T>,
    from: T,
    shown: T,
    heading: T,
    target: Option<T>,
    base: T,          // the lane's own copy of the resting value
    elapsed: f32,
}
```

`advance` loses its `elements` and `store` parameters and its
`E: Element<H>` bound. The push is one line:

```rust
(self.write)(&mut Patch::new(world, node, theme), &self.shown);
```

**The base.** Today `advance` re-reads the base from the element every
frame, because "the base can move mid flight". It moves only when a
binding on the same field fires, and a structural rebuild tears the
lane down, so: when a binding fires and `lanes` has an entry for
`(node, field_id)`, it calls `lane.rebase(new)`. The lane carries its
own `base: T`, seeded from the cascade value at creation
(`insert_travel` already reads it there once).

### 3. Trim `Element::patch`

Once both callers are off it, the recursive own-field walk and the
generated `patch` impl can go. Keep the child-hop dispatch only if
something still needs a path walk; keep the `despawn` walk. `Fields` /
the `*Field` enum stay - cursors and `field_id`/`field` still use them.

## Caveats

### The retained element is still a base-value store

`Fynix::element(node)` is public and documented as "always reads the
latest" - editor code and tests read field values back through it. If
bindings stop writing the struct, that getter returns the cascade-time
snapshot instead.

Recommended: **keep the binding's `*field = new` write** (step 1 above
keeps it). It is one move, it keeps `Fynix::element` honest, and it
keeps the door open for anything else that reads the table. Only the
`element.patch` *walk* is removed. Lanes never need the write - they
don't own the base, they borrow a copy.

The aggressive alternative - drop the struct write, let
`Fynix::element` return the snapshot or delete it - is a separate
decision, not required for the speedup.

### Same-flush ordering: binding then lane

A lane must re-assert every frame even when `shown` has not moved
(today's `// Pushed even when unmoved` comment). A binding on the same
field calls its writer earlier in the flush and puts the raw base on
the component; the lane has to write after it. Flush order stays
bindings, then lanes.

### Cached node lifetime

`(owner_node, writer)` is valid for the binding's or lane's life
because a structural rebuild of the owning element recreates its
bindings and lanes. This has to actually hold: if a binding can
outlive a rebuild of an ancestor element, the cached node dangles.
Confirm before shipping.

### Missed rebase releases to a stale base

If a binding fires without notifying a co-located lane, a later
`aim(None)` animates the field back to the wrong resting value. The
notify is a new invariant the binding path must not drop.

### Mid-flight rebase semantics (unchanged)

Aiming (`target = Some`) ignores the base. Releasing (`target = None`)
heads to the current base. The rebase-notify keeps both, same as the
live re-read does today.

### Writers stay full-writes

A writer runs once per field at build and again on each patch; given
the same value it must land the same component state. Already true for
every writer in `patch.rs`.

## Pros and cons

**Pros**

- Binding fire drops from O(depth) virtual `patch` calls + O(fields)
  scan to one component write.
- Lane advance sheds the typed table lookup, the element borrow and the
  walk - just an interpolation and a write. This is the per-frame path.
- `Lane` stops depending on `Element`; `advance` sheds two parameters
  and a bound. The trait gets small enough to reconsider boxing.
- Transition correctness no longer rides on the replace/patch/restore
  dance being exactly right.
- `insert_travel` keeps its one-time base read; nothing reads the
  element per frame any more.

**Cons**

- Macro work: emit `OwnField<H>` impls per field / terminal cursor
  segment.
- New invariant: a binding must `rebase` a co-located lane. A missed
  call is a silent wrong-resting-value bug.
- Bind-time cost moves up front: one `store` walk per binding. Net win
  unless a binding is created and never fires.
- A fn pointer and a node cached per binding and per lane - negligible.
- Two dispatch paths coexist during migration until lanes move over.

## Open questions

- Does anything call `Element::patch` besides `ui.rs` and `lanes.rs`?
  (Current grep: no.)
- Besides `Fynix::element(node)`, who reads `records.elements`?
  `Build::child` uses `store`, not the table - confirm nothing else in
  the editor does.
- Can a binding or lane outlive a rebuild of an ancestor element?
- Does interaction/`aim` wiring touch the element, or only lanes and
  the world?

## Work items

- [ ] `OwnField<H>` trait in `fynix`; `#[element]` emits impls per own
      field.
- [ ] Cursor exposes the terminal field's writer; `bind` resolves
      `(owner_node, writer)` at bind time and stores it on `Binding`.
- [ ] Binding fire path calls the writer directly; keep `*field = new`
      for `Fynix::element`.
- [ ] New `Travel` repr; `Lane::advance` drops `elements` / `store` /
      the `Element` bound.
- [ ] `Binding` -> `Lane` rebase notification keyed on
      `(node, field_id)`.
- [ ] Delete `Element::patch`'s own-field walk once both callers are
      off it; keep child-hop only if still needed, keep `despawn`.
- [ ] Audit `records.elements` readers; leave the table for
      `Fynix::element` and despawn, out of the reactive path.
- [ ] Bench a splitter drag and a playing timeline, before and after.
