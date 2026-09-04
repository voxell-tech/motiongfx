# Transition despawn

Built. This started as a plan to replace the sweep with an explicit
kernel hook; the implementation found that a hook cannot cover the
whole problem, and that the sweep's real cost was somewhere else. What
follows is what shipped and why.

## What the sweep actually costs

`AnimTable::retain` walks a key set once per flush, dropping rows whose
nodes the backend no longer has. The first cut of that set held *every
field that had ever moved* and never shrank, so a long-running tree
paid a growing walk every frame to catch a rare event.

The fix was not to remove the sweep but to shrink what it walks: a key
goes in when a leg starts and comes out when `tick` drops the settled
row, so the set is exactly the fields moving right now. A resting tree
holds none, and the per-flush sweep is free.

`reresolve` only records a key once `retarget` has actually left a row
behind - a retarget that finds nothing to move (the destination is
already where the field rests) starts no leg and must not be tracked.

## Why not an explicit hook

The plan was `TransitionTable::despawn(node, kind)`, called on the
kernel's despawn walk, reading the element type's animated fields to
remove each `FieldKey::new(node, field)`. Two things stop that from
replacing the sweep:

- **fynix does not own every despawn.** `clear_children` is the only
  path fynix drives, and the app is free to despawn a node behind its
  back - `Host::exists` is how the kernel finds out at all. A sweep is
  required for that case whatever else exists.
- **`clear_children` has no `Records`.** Threading them through, then
  walking the dying subtree to look up each node's kind, is real
  machinery for the half of the problem the sweep already covers at no
  cost now that it only walks moving fields.

So there is no hook. If a profile ever shows the sweep mattering, the
hook is still available for the fynix-driven half, but it would be an
addition to the sweep rather than a replacement.

## What is freed

- **`Transition` rows** - by the sweep, keyed off the moving set.
- **`Active<T>` tag slots** - columns on the element's own row in
  `ElementTable`, so `remove_row` in the `element_nodes` sweep takes
  them along. Nothing extra to call.
- **Per-type tables** - `Source` pool entries, `animated(kind)`, the
  tick and retarget registries: keyed by element `TypeId`, shared by
  every instance, never freed. Despawning one button must not disturb
  the button type's recipe. Bounded by the count of distinct element
  types built.

`AnimTable::forget` drops all of it at once, for a theme change: a
registration bakes in the tweens the theme named, and the rows point
into the sources being replaced.

## Covered by

`crates/fynix/tests/tags.rs`:

- `settled_field_holds_no_row` - arrival drops the row.
- `despawning_drops_the_row` - a node dying mid-leg is swept.
