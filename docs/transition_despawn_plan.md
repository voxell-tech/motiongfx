# Transition despawn: an explicit hook

Planning notes. The `tick`-time liveness check in
`transition_tag_model.md` is a placeholder. This is what should
replace it, and what it needs from the kernel.

## Why not a lazy sweep

Two lazy options are on the table and both are wrong:

- the shipped `TransitionTable::retain(|node| alive(node))`, walking
  the whole table for corpses, and
- a `!elements.contains(key.node)` check at the top of the `tick`
  loop.

- **Despawn is an event, not a state to poll.** The kernel knows the
  exact moment a node dies. Scanning for the aftermath is backwards.
- **The cost lands on the wrong path.** The check taxes every moving
  transition every frame — or `retain` walks every row periodically —
  to catch a rare event.
- **Coupled to the tick loop.** If ticking is skipped — nothing
  animating, playback paused, the kernel idle — dead rows linger until
  something iterates them.
- **One frame of lag.** A despawned node's last value can be written
  once more before the row goes.

## The hook

```rust
impl TransitionTable<H> {
    /// Drop every per-node row for `node`. Called by the kernel's
    /// despawn path, once per node, before the element record is torn
    /// down.
    fn despawn(&mut self, node: H::Node, kind: TypeId);
}
```

- `kind` is passed in because the caller reads it off the element
  record, which is about to disappear — the hook must run *before*
  that teardown.
- Idempotent: a second call for a node with no rows is a no-op.

## Finding the rows without a per-node index

The table is `TypeTable<FieldKey<H>>` keyed by `(node, field)`. There
is no "all keys for node N" query, and a side
`HashMap<Node, Vec<FieldKey>>` would tax every transition spawn and
settle.

Not needed — the per-type registration already enumerates the
element's animated fields:

```rust
fn despawn(&mut self, node: H::Node, kind: TypeId) {
    for field_lines in animated(kind) {
        self.table.remove_row(&FieldKey::new(node, field_lines.field));
    }
}
```

`remove_row` drops the row across every value-type column — the same
behaviour the shipped `TransitionTable::insert` relies on to replace a
field's transition — so the hook needs neither each field's value type
nor a check that a row exists.

`animated(kind) -> &[FieldLines<H>]` is the per-type list already
built for `set_tag` (`transition_tag_model.md`); despawn just reads
`.field` off each entry.

Cost: O(animated fields of that element type), once, at despawn.
Nothing per frame.

## The despawn sequence

Per node, on the kernel's despawn walk:

1. `kind = elements.kind(node)`
2. `transitions.despawn(node, kind)` — drops the `Transition` rows
3. tear down the element record — drops `Active<T>` with it

`Active<T>` needs no explicit call as long as step 3 always runs and
frees the record's tag slots. If tag state ever leaves the element
record, it gets its own line in step 2.

## Subtree despawn

The kernel already walks a despawned subtree node by node (shipped
`Host::despawn` + `Host::children`). The hook fires once per node on
that walk — no separate recursion here.

## Same-flush `set_tag` + `despawn`

If `set_tag(node, ..)` and `despawn(node)` land in the same flush, a
`set_tag` processed after the `despawn` re-inserts rows for a dead
node. Options:

- the kernel guarantees `despawn` is the last word for a node in a
  flush, or
- `set_tag` drops work for a node the `despawn` walk has already
  marked.

Pin this once the flush order is settled. Until then, `tick` keeping a
liveness check as a backstop is acceptable — but it is a backstop, not
the mechanism.

## Not freed, and correct

`Source` pool entries, `animated(kind)`, the `retarget` registry — keyed by
element `TypeId`, shared by every instance. Despawning one button must
not disturb the button type's recipe. These live for the process,
bounded by the count of distinct element types built.

## Migration

Replaces `TransitionTable::retain`. With the hook in place `tick`
assumes every row it iterates is live; the `elements.contains` check
in the model doc comes out (or stays only as the documented backstop
above).

## Open

- **Kernel hook point.** Where the despawn walk calls out — a trait
  method, a callback list, a direct `TransitionTable` call from the
  kernel's despawn fn.
- **`animated(kind)`** — the per-type `FieldLines` list; populated in
  the same lazy step as `Source`s.
- **`elements.kind(node)` at despawn time** — must resolve while the
  record still exists; confirm the despawn sequence orders it first.
- **Flush order** for the `set_tag` / `despawn` race above.
