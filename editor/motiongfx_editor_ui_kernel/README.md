# motiongfx_editor_ui_kernel

A reactive UI kernel with no renderer, no layout engine, and no
elements of its own. It owns a tree of *nodes* and keeps them in sync
with some external state; what a node actually **is** belongs entirely
to the backend.

`no_std` (needs `alloc`).

## The two primitives

|          | Fires on  | Does                               |
| -------- | --------- | ---------------------------------- |
| `watch`  | predicate | clears its children, rebuilds them |
| `bind`   | predicate | writes one field in place          |

That split is the whole design. A watcher handles **structure** (rows
appeared, a panel was closed); a binding handles **values** (a label's
text, a panel's ratio). Reaching for a watcher where a binding belongs
is the usual mistake: rebuilding throws away scroll offsets, focus,
in-flight drags and anything else the nodes were holding.

The rule of thumb is cost. A binding's predicate runs every flush, so
it must be cheaper than the write it guards. Comparing a change tick is
cheap; recomputing the value you were about to write is not: at that
point a plain loop is the better tool.

## The `Host` trait

Everything the kernel needs from a backend, and nothing more:

```rust,ignore
pub trait Host: Send + Sync + 'static {
    type Node: Copy + Eq + Hash + Send + Sync + 'static;
    type World: 'static;
    type Widget: Send + Sync + 'static;

    fn spawn(world: &mut Self::World, parent: Self::Node) -> Self::Node;
    fn apply_widget(world: &mut Self::World, node: Self::Node, widget: Self::Widget);
    fn exists(world: &Self::World, node: Self::Node) -> bool;
    fn children(world: &Self::World, node: Self::Node) -> Vec<Self::Node>;
    fn despawn(world: &mut Self::World, node: Self::Node);
}
```

`Node` is opaque: the kernel only stores and compares it. `Widget` is
opaque too: the kernel hands it straight back to `apply_widget`, so a
backend can make it a scene, a closure, an enum of both.

Anything needing concrete types lives in the adapter, not here. The
kernel stores only `bind_raw`, whose closures the caller has already
erased; a typed `bind::<Component>` is an extension trait on the
backend side, where the type is still in scope.

## Building is two-phase

A builder **reads** state and **returns a description**; the kernel
applies it afterwards.

```rust,ignore
fn build(world: &World, ui: &mut Ui<H>)
```

This isn't ceremony, it's what makes the borrows work. A builder that
both read the world and spawned into it would need `&World` and
`&mut World` at once. Splitting them means `Ui` can accumulate while
the world is only borrowed shared, and the kernel does every mutation
in one pass.

## Declaring a tree

```rust,ignore
ui.node(widget)                  // spawn a node, filled by a widget
  .with(|ui| { ... })            // static children
  .watch(changed, build)         // children rebuilt when `changed` fires
  .bind_raw(changed, apply);     // a field that tracks state

ui.group()                       // a node with no widget
ui.bind_raw(changed, apply)      // a node that only carries a binding
```

Everything hangs off a node, so a builder reads as "make this, then say
how it reacts". Use `.watch(..)` *or* `.with(..)` on a node, not both:
a fire clears whatever children it has.

`watch` nests. A watcher declared inside a build is re-registered every
time that subtree rebuilds, so nested reactivity survives an outer
rebuild without anything re-registering it by hand.

## Predicates

```rust,ignore
FnMut(&World, Node) -> bool
```

`FnMut` because a predicate usually diffs against what it last saw, and
that value has to live somewhere: a tick, a hash, a previous value.

It takes its node so it can read state *relative to itself*: a parent's
computed size, a sibling's scroll offset. A builder can't know entity
ids that don't exist yet, so without this, node-relative reactivity
would mean searching the world for a marker on every poll.

**A predicate must be called exactly once per flush.** A stateful one
consumes its own signal, so a second call in the same frame reports
"unchanged". That rules out probing them for logging or re-evaluating
them in a debug pass.

## The flush

`Kernel::flush(&mut World)` runs every watcher and binding whose
predicate fires. Records live in the `Kernel`, not the world, so
`&mut Kernel` and `&mut World` are already disjoint, so nothing has to
be taken out of the world and put back.

Cleanup is by polling, via `Host::exists`:

- A watcher's root is checked as the loop reaches it, not once up
  front. An earlier watcher's rebuild can despawn a later one's root,
  and building into a freed handle would reparent onto a dead node.
- Bindings for despawned nodes are dropped after the watcher pass.
- A rebuild forgets its old subtree's bindings before respawning.
  Without that, each rebuild leaks one binding per node and the stale
  ones keep firing.

A watcher registered during a flush takes effect on the *next* one.

## Not here

**Keyed reconciliation.** A rebuild is all-or-nothing. Scoping is the
mitigation: a watcher around just the rows costs far less than one
around the whole panel. Keys are only worth it when the same node must
survive a rebuild with new contents.

**Transitions.** Animating a property between states needs a resting
value and per-node ownership of the animated field; neither is
modelled.

**Relationship arity.** `children` returns a `Vec` and `spawn` takes a
single parent, so one ordered child list per node is assumed. Backends
whose trees aren't shaped that way will need this widened.
