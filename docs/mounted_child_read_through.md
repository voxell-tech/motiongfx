# Mounted children: read through the parent, don't clone

Design conversation, not built. Proposes replacing `Records::mount_child`'s
clone with a pointer back to the parent.

## Where the current model falls short

A `#[elem(child)]` field (`Button.icon: Option<Icon>`) is a real value living
inside its parent's struct. But `anim`'s `Access` fns read a field's owner via
`elements.get::<Icon>(&node)` - a lookup keyed on the *child's own node*, not
the parent's. So when the child's node is built, `mount_child` clones the
value into a second home at that node:

```rust
records.mount_child(child, ::core::clone::Clone::clone(elem));
```

Costs:

- **A clone on every parent rebuild.** For `Icon`/`Label`, that's a `String`
  allocation, not just a bit-copy.
- **Every `#[element]` struct now derives `Clone`**, forced by the macro so
  any element can be someone's child. A plain-data field that used to get
  away without `Clone` (`crates/fynix/tests/elements.rs`'s `Border`) no
  longer can.
- **The clone goes stale.** A child field driven by `.bind()` updates the
  parent's copy (`elements.get_mut::<Parent>(&parent_node)`), not the
  child's mounted snapshot. Nothing currently writes the second copy back in
  sync.

## The pointer model

Store a pointer back to the parent instead of a copy of the value:

```rust
/// Where a mounted child's real value lives: not here, at `parent`.
struct MountedIn<H: Host, C> {
    parent: H::Node,
    read: fn(&ElementTable<H>, H::Node /* parent */) -> Option<&C>,
}
```

`read` is generated once, at the exact site that already knows both concrete
types - the parent's own `#[elem(child)]` codegen:

```rust
// Button::build, for `icon: Option<Icon>`
|elements, parent| {
    resolve::<H, Button>(elements, parent)
        .and_then(|button| button.icon.as_ref())
}
```

`parent` and `read` are both plain `Copy` data - a `Node` and a function
pointer, no captures, so no `Box<dyn _>` and no allocation. `mount_child`
stops taking the child's value at all:

```rust
pub fn mount_child<C: Send + Sync + 'static>(
    &mut self,
    node: H::Node,
    parent: H::Node,
    read: fn(&ElementTable<H>, H::Node) -> Option<&C>,
) {
    self.elements.insert(node, MountedIn { parent, read });
    self.element_nodes.insert(node, TypeId::of::<C>());
}
```

## Nested children need recursion, not one hop

A child can itself be mounted under another child - `Tab.close: Option<Button>`,
`Button.icon: Option<Icon>` makes `Icon` a grandchild of `Tab`. A `MountedIn`
that does `elements.get::<Button>(&button_node)` directly fails here: `Button`
is *also* a mounted child, so there is no real `Button` value at
`button_node` either, only its own `MountedIn<H, Button>` pointing at `Tab`.

The fix is a shared, generic resolver that both the top-level and the
mounted case go through:

```rust
fn resolve<H: Host, E: Send + Sync + 'static>(
    elements: &ElementTable<H>,
    node: H::Node,
) -> Option<&E> {
    elements.get::<E>(&node).or_else(|| {
        let mount = elements.get::<MountedIn<H, E>>(&node)?;
        (mount.read)(elements, mount.parent)
    })
}
```

A top-level element (built via `Ui::elem`, which still inserts the real
value) resolves on the first branch and stops - no indirection added to the
common case. A mounted child falls through to its `MountedIn` entry, whose
`read` fn calls `resolve::<H, Parent>` in turn, chasing up however many hops
the tree actually has. Depth is small in practice (2-3 levels), and the walk
only runs on a tag re-resolve or a moving field's tick, not every field every
frame.

## What changes

- **`records.rs`** - add `MountedIn<H, C>` and `resolve<H, E>`; change
  `mount_child`'s signature to `(node, parent, read)` as above.
- **`fynix_macros::element`** - the `#[elem(child)]` build codegen emits a
  `read` closure instead of cloning `elem`; the blanket `#[derive(Clone)]`
  and the `+ Clone` bound on child types both go away.
- **`anim.rs`'s `anim_field` codegen** - every generated `Access` closure
  (`on(...)` lines and a field's base) switches from
  `elements.get::<Name>(&node)` to `resolve::<H, Name>(elements, node)`.
- **Tags are untouched.** `Hovered`/`Pressed` are stored directly on
  whatever node actually receives the pointer event
  (`elements.insert(node, tag)`), independent of which node holds the real
  element value. `tagged::<H, T>` keeps reading `elements.get::<T>(&node)`
  as it does today.
- **Despawn is untouched.** `MountedIn<H, C>` is just another column on the
  child's row; the existing sweep (`element_nodes.retain` +
  `elements.remove_row`) already drops every column for a dead node.
- **`Store` is untouched.** It still maps parent+field to child node; that
  lookup is how the parent's build hook gets the `parent` node to hand
  `mount_child` in the first place.

## What this fixes for free

A `bind`-driven child field stops going stale: since `resolve` always reads
the parent's live value, there is no second copy left to drift out of sync.
The workaround this doc's predecessor left open (`bind`'s apply writing
through to a child's mounted snapshot) is no longer needed - there is no
snapshot to write through to.

## Open questions

- Where `resolve` lives - `records.rs` beside `ElementTable`, or `anim.rs`
  beside the other `Access`-adjacent machinery. It needs to be `pub`, since
  macro-generated code in every downstream crate calls it.
- Whether a top-level element should also get a (trivial, self-pointing)
  `MountedIn` for uniformity, or whether leaving it out - relying on
  `resolve`'s first branch - is the better default. Leaning towards leaving
  it out: it is the common case, and skipping the indirection there is free.
