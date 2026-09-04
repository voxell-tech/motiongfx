# Transition: the tag push model

How a field travels, and what makes it. Built; `crates/fynix/src/anim.rs`
is the runtime and `crates/fynix/tests/tags.rs` covers it. Replaced the
`aim` / `aim_on` model and the resolver sketch in
`transition_redesign_handover.md`, both now deleted.

## Shape in one paragraph

A field declares, in its `#[elem(anim(...))]` attribute, an ordered
list of tagged values: "when tag X is active, animate to value Y over
Z ms," highest priority first. The backend maintains a **set** of
active tags on a node — `ui.node(id).set_tag(x)` /
`.unset_tag::<X>()` — one call per raw event, no state machine. Each
call re-resolves the node's animated fields: each field walks its list
top-down and takes the first line whose tag is active, else its base.
The resolved destination spawns or retargets a `Transition`; a
per-frame `tick` interpolates the live ones and deletes each on
settle. A resting UI ticks nothing.

**Priority, not replacement.** Tags are independent and stack. Press
does not wipe hover: a field that only lists `on(Hovered)` keeps its
hover value while `Pressed` is also active, and a field listing both
takes whichever ranks higher. Removing `Pressed` falls back to the
still-active `Hovered`.

## Why not the shipped model

`aim` froze the target at build, kept a hand-maintained `base` copy,
and iterated every field with a transition every frame. See the
handover doc's "Where the current model falls short".

## Why not the handover's resolver model

The handover had each field carry a `resolve: fn(WorldNodeRef) -> T`
that read interaction state out of the world like a binding. The push
model drops that: the backend already knows when a pointer enters or a
toggle flips (bevy picking is evented), so it calls `set_tag`
directly. No polling, no per-frame world reads, no `Host::Interaction`.

## Tag types

A tag type is **any `Copy + PartialEq + Send + Sync + 'static`**. A
`Tag` trait exists only to name that bound, blanket-impl'd so every
qualifying type — host enum, `bool`, third-party type, plain marker —
is a tag with no impl to write and no macro:

```rust
pub trait Tag: Copy + PartialEq + Send + Sync + 'static {}
impl<T: Copy + PartialEq + Send + Sync + 'static> Tag for T {}
```

`set_tag<T: Tag>` / `Active<T: Tag>` read the same as the bare bound.

```rust
#[derive(Clone, Copy, PartialEq, Eq)] struct Hovered;
#[derive(Clone, Copy, PartialEq, Eq)] struct Pressed;
#[derive(Clone, Copy, PartialEq, Eq)] enum ToggleState { On, Off }
```

Each type is an independent slot — a node can be `Hovered` and
`Pressed` and `On` at once. That independence is what lets a
hover-only field keep its hover value while `Pressed` is also set, so
model hover/press as **two types, not one enum** (a single
mutually-exclusive `Interaction` enum would make press replace hover
and drop hover-only fields to base). fynix never looks inside a tag —
it stores `Active<T>` and the field predicates ask "is it set" /
"does it equal this variant".

## Storage

Almost everything is per element type and lazily populated. Per node
there is only a transient `Transition` row while a field moves, and
the `Active<T>` slots on the element record.

### Per node

- **`Transition<H, T>`** — in `TypeTable<FieldKey<H>>`, one column per
  value type, keyed by `(node, field)`. Transient; `tick` removes the
  row on settle. A resting UI has no rows.
- **`Active<T>`** — one `Option<T>` slot per tag type the node has
  ever carried, on the element record in the `ElementTable`. Several
  are `Some` at once.

### Per element type — lazily registered on first build

The first `build()` for an element type registers its animation data,
guarded by a `HashSet<TypeId>` of seen types. All of it is identical
across instances.

- **`Source<H, T>`** in the `TypePool` — one per `(field,
  destination)`, where a destination is an `on(...)` line *or the
  field's base*. Holds the value accessor, the tween, and the field's
  `patch` fn (repeated across the field's sources — a handful of
  8-byte pointers). `pool.insert` runs here; the `PoolKey`s go
  straight into `FieldLines`.
- **`animated(kind: TypeId) -> &[FieldLines<H>]`** — every animated
  field of the type, with its priority-ordered lines:

  ```rust
  struct FieldLines<H: Host> {
      field:    FieldId,
      value_ty: TypeId,                                  // to pick the retarget fn
      base:     PoolKey,                                 // Source when nothing matches
      lines:    Box<[(ActiveFn<H>, PoolKey)]>,           // priority order, first match wins
  }

  type ActiveFn<H> = fn(&ElementTable<H>, H::Node) -> bool;
  ```

  Each `ActiveFn` is one generated predicate: `on(Hovered, ..)` →
  `|e, n| e.tagged_by::<Hovered>(n)`; `on(ToggleState::On, ..)` →
  `|e, n| e.tag::<ToggleState>(n) == Some(ToggleState::On)`. The tag
  type never has to be named in storage — the predicate closes over
  it.

  Backed by `HashMap<TypeId /*element*/, Box<[FieldLines<H>]>>`,
  written once, read-only after.

- **`retarget::<H, T_value>`** pushed into the value-type registry,
  if absent.

### Value-type registry

`Vec<(TypeId, RetargetFn<H>)>`, keyed by `TypeId::of::<T_value>()` —
the shipped `TransitionTable::ticks` pattern.

```rust
type RetargetFn<H> = fn(
    &ElementTable<H>, &mut TypeTable<FieldKey<H>>, &TypePool,
    FieldKey<H>, PoolKey, PoolKey,   // from_key, to
);
```

## `Access` — the pooled value accessor

```rust
type Access<H, T> = for<'a> fn(
    &'a ElementTable<H>, H::Node,
) -> Option<&'a T>;
```

`for<'a>` because the returned `&T` borrows from the passed
`&ElementTable`; `Option` because the node may not hold that element.
`Copy + 'static + Send + Sync`, so it sits in a pool column unwrapped.

`read = <field>` in each `on(...)` is read as `fn(&E) -> &T`; the
macro wraps the element downcast so the column keys on `T` alone.
`read = Self::method` passes the method straight through. The base
source is the field itself:

```rust
// on(Pressed, read = active_bg)
//   |elements, node| &elements.get::<ButtonElem>(node).unwrap().active_bg
// base for `bg`
//   |elements, node| &elements.get::<ButtonElem>(node).unwrap().bg
```

## `Source`

```rust
struct Source<H: Host, T> {
    access: Access<H, T>,
    tween:  Tween<T>,          // anim(ms) default, overridden by on(.., ms)
    patch:  fn(&mut Patch<H>, &T),
}
```

One per `(field, destination)`, base included. `tween` and `patch`
live here — there is no per-field recipe left to carry them.

## `Transition` — transient, one per *moving* field

```rust
struct Transition<H: Host, T> {
    tween:    Tween<T>,    // this leg's tween; duration is cut on reverse
    from:     T,           // the leg's start value
    elapsed:  Duration,
    to:       PoolKey,     // the destination Source
    departed: PoolKey,     // the Source we left — reverse iff to == departed
}
```

- No `Tag` generic. `to` / `departed` are `PoolKey` — `Copy +
  PartialEq`. Base is a `Source` like any other, so no `Option`.
  Identity only — `tick` fetches the `Source` at `to` each frame for
  its `access` and `patch`.
- `from` is the one value that must be stored: after an interruption
  the leg starts from a mid-interpolation value in no field and no
  `Source`.
- The destination value is *not* stored — read live through the
  `Source` accessor every frame, so a `bind` that changes it
  mid-flight is free.

### `Tween<T>` — unchanged from shipped

`Duration` + `EaseFn` + `LerpFn<T>`. `duration` is `Duration` (done on
this branch). `at(elapsed) -> f32` reads `self.duration`.

## The per-frame tick

One `tick::<H, T>` per value type, registered like the shipped
`TransitionTable::ticks`.

```rust
fn tick<H, T>(
    table: &mut TypeTable<FieldKey<H>>,
    pool: &TypePool,
    dt: Duration,
    world: &mut H::World,
    elements: &ElementTable<H>,
    theme: &H::Theme,
) where
    H: Host,
    T: Clone + Send + Sync + 'static,
{
    let mut done: Vec<FieldKey<H>> = Vec::new();   // settled this frame
    for (key, transition) in table.iter_mut::<Transition<H, T>>() {
        transition.elapsed += dt;
        let source = pool.get::<Source<H, T>>(&transition.to).unwrap();
        let to = (source.access)(elements, key.node); // &T
        let value = (transition.tween.lerp)(
            &transition.from,
            to,
            transition.tween.at(transition.elapsed),
        );
        (source.patch)(&mut Patch::new(world, key.node, theme), &value);
        if transition.elapsed >= transition.tween.duration {
            done.push(*key);
        }
    }
    for key in done {
        table.remove_row(&key);
    }
}
```

The accessor returns `&T`, so `tick` never clones the destination —
only `from` is owned, cloned at spawn and each interruption.

## The `set_tag` flow

Re-resolve every animated field of the node against the tag set. A
field with no line for the changed type resolves to the same key and
`retarget` returns early — a few wasted predicate calls, nothing else.

```rust
fn set_tag<T>(
    elements: &ElementTable<H>,
    table: &mut TypeTable<FieldKey<H>>,
    pool: &TypePool,
    node: H::Node,
    new: T,
) where T: Tag {
    let kind = elements.kind(node);

    // resting keys under the OLD tag set, before the write
    let from: Vec<PoolKey> =
        animated(kind).iter().map(|f| f.resolve(elements, node)).collect();

    elements.set_tag::<T>(node, new);

    for (f, from_key) in animated(kind).iter().zip(from) {
        let to = f.resolve(elements, node);   // under the NEW tag set
        (retarget_of(f.value_ty))(
            elements, table, pool, FieldKey::new(node, f.field), from_key, to,
        );
    }
}
```

```rust
impl<H: Host> FieldLines<H> {
    fn resolve(&self, elements: &ElementTable<H>, node: H::Node) -> PoolKey {
        self.lines.iter()
            .find(|(active, _)| active(elements, node))
            .map_or(self.base, |&(_, key)| key)
    }
}
```

- `set_tag::<T>` sets `Active<T> = Some(new)` — replacing any previous
  value *of that type*. Other types' slots are untouched.
- `unset_tag::<T>(node)` clears `Active<T>` and runs the same
  re-resolve. Fields fall to their next-highest active line, or base.
- `clear_tags(node)` clears every slot — for `Fynix::despawn` or a
  wholesale reset.

### Backend forwards events, no state machine

fynix does not know what `Hovered` means. The backend (`bevy_fynix`)
maps events one-to-one, because tags stack:

| event | call |
| --- | --- |
| `Over` | `node.set_tag(Hovered)` |
| `Out` | `node.unset_tag::<Hovered>()` |
| `Down` | `node.set_tag(Pressed)` |
| `Up` / `Cancel` | `node.unset_tag::<Pressed>()` |

No `{ over, pressed }` bookkeeping: on `Up` the node is still
`Hovered` (never unset), so `bg` falls back to its hover line on its
own. `ToggleState`, focus, etc. are each their own independent
mapping.

### `retarget` — one generic fn per value type

```rust
fn retarget<H, T>(
    elements: &ElementTable<H>,
    table: &mut TypeTable<FieldKey<H>>,
    pool: &TypePool,
    key: FieldKey<H>,
    from_key: PoolKey,     // resting Source; used only when no leg is running
    to: PoolKey,
) where H: Host, T: Clone + Send + Sync + 'static {
    let tween = pool.get::<Source<H, T>>(&to).unwrap().tween;

    match table.get_mut::<Transition<H, T>>(&key) {
        Some(transition) => {
            if to == transition.to { return; }
            let access = pool.get::<Source<H, T>>(&transition.to).unwrap().access;
            let here = (transition.tween.lerp)(
                &transition.from,
                access(elements, key.node),
                transition.tween.at(transition.elapsed),
            );
            let reverse = to == transition.departed;   // back where this leg started
            let spent   = transition.elapsed;
            transition.departed = transition.to;
            transition.to       = to;
            transition.from     = here;
            transition.elapsed  = Duration::ZERO;
            transition.tween    = tween;
            if reverse {
                transition.tween.duration = tween.duration.min(spent);
            }
        }
        None => {
            if to == from_key { return; }              // already at rest there
            let access = pool.get::<Source<H, T>>(&from_key).unwrap().access;
            let from = access(elements, key.node).clone();
            table.insert(key, Transition {
                tween, from,
                elapsed: Duration::ZERO,
                to,
                departed: from_key,
            });
        }
    }
}
```

## Reverse and the duration cut

`departed` is the `Source` the current leg started from. A new
destination equal to it means "going back where we came from":

```
tween.duration = tween.duration.min(elapsed);   // only as long as we had travelled
elapsed = 0;                                     // fresh tween over the shorter window
```

An ease-out then settles gently into the rest state rather than
playing its tail. Trace, `bg` with `on(Pressed)` then `on(Hovered)`,
tween duration `d`:

1. Settled at base (no row).
2. `set_tag(Hovered)` → spawn. `from = base`, `to = hover_key`,
   `departed = base_key`. Ease-out in.
3. `set_tag(Pressed)` at 40%. `to = press_key`, `departed = hover_key`
   (was heading there). Not a reverse — fresh leg from `here` to
   press over the press tween.
4. `unset_tag::<Pressed>()` shortly after, at 25% of the press leg.
   `bg` re-resolves: `Pressed` gone, `Hovered` still active →
   `to = hover_key`. `to == departed` ⇒ reverse. `tween.duration`
   cut to that 25%, `elapsed = 0`, eases back to hover.
5. `unset_tag::<Hovered>()` → `to = base_key`. Not the reverse of the
   step-4 leg (its `departed` is `press_key`) — full ease home.

No reversing-shortening factor, no linear-progress float, no bezier
reflection. Double reverses compound because each leg recomputes from
its own `elapsed`.

**Deferred:** true velocity continuity at the reversal instant (CSS
flips the timing function). The fresh-tween-over-cut-window is the
approximation; revisit only if an aggressive ease looks wrong.

## Despawn and lifetime

Only per-node state is freed. Plan: `transition_despawn_plan.md`.

- **`Transition` rows** — dropped by an explicit
  `TransitionTable::despawn(node, kind)` on the kernel's despawn walk,
  removing `FieldKey::new(node, field)` for each animated field of the
  type. The `!elements.contains` check in `tick` is a backstop until
  the hook lands.
- **`Active<T>` slots** — co-located with the element record, dropped
  by `ElementTable::despawn`.
- **Per-type tables** — `Source` pool, `animated(kind)`, the
  `retarget` registry — keyed by element `TypeId`, shared by every
  instance, never freed. Bounded by the number of distinct element
  types built.

## Tag on the element

```rust
elements.tag::<ToggleState>(node)     -> Option<ToggleState>
elements.tagged_by::<Hovered>(node)   -> bool
elements.set_tag::<T>(node, tag)      // written by set_tag
elements.clear_tag::<T>(node)         // written by unset_tag / clear_tags
```

The `ActiveFn` predicates call these. Not consulted per frame.

## Attribute

```rust
#[element]
struct ButtonElem {
    #[elem(patch = PatchBg, anim(duration = theme.motion.interact,
        on(Pressed, read = active_bg),              // higher priority
        on(Hovered, read = hover_bg, ms = 200),
    ))]
    bg: Color,

    #[elem(patch = PatchBorder, anim(ms = 120,
        on(Hovered, read = hover_border),           // hover only — press never touches it
    ))]
    border: Color,

    // Bare fields — addressable, no writer. Element state the lines
    // read through; a style or a call site sets them, nothing draws them.
    hover_bg:     Color,
    active_bg:    Color,
    hover_border: Color,
}
```

- `=` only, no `=>`.
- `on(...)` order **is** the priority — first match wins, per field.
  No global tag rank; a field may list several tag types, and two
  fields may order the same tags differently. The list is totally
  ordered, so `resolve` never has a tie.
- `ms = 120` at `anim` level is the default leg duration; `duration =
  <expr>` names an exact `core::time::Duration` (a theme value serves
  as well as a literal). A per-`on` `ms` / `duration` overrides it.
- `read = <sibling field>` reads the field in place, so it stays
  theme-driven; `read = Self::method` names an `fn(&Self) -> &T` for a
  value the element works out rather than stores. A bare name is a
  field, a qualified path a method.

### Field taxonomy

| form | backend writer | addressable | meaning |
| --- | --- | --- | --- |
| `#[elem(patch = X)]` | yes | yes | a drawn property |
| bare (no `#[elem]`) | no | yes | element state: feeds `read =`, not drawn |
| `#[elem(ignore)]` | no | no | build-time scratch only |
| `#[elem(child)]` | — | — | sub-element |

`anim(...)` requires `patch = ...`: an animation must be able to
write through. A bare field can be a `read =` destination but cannot
be animated directly.

## Styles

A `Style` never touches tags or transitions. It sets the bare fields
(`hover_bg`, ...) with ordinary assignment, exactly as any field
today. The `Access` fns read them live.

```rust
impl Style for Accent {
    fn apply(self, b: &mut ButtonElem, theme: &Theme) {
        b.bg           = theme.surface;
        b.hover_bg     = theme.surface.lift(0.05);
        b.active_bg    = theme.surface.lift(0.10);
        b.hover_border = theme.accent;
    }
}
```

## `bind`

`anim` on a field ⇒ the animation is the sole writer through the
field's `patch` tag. A `bind` on a source field (`hover_bg`) just does
`*field = new`; the accessor picks it up on the next frame. `bind`
must accept fields with no `FieldPatch` tag (the bare source fields) —
same requirement the handover noted.

## To derive / open

- **`retarget_of(TypeId)`** — the value-type registry lookup; how the
  macro pushes `retarget::<H, T>` into it.
- **Lazy registration.** First `build()` per element type inserts
  `Source`s (base + every `on`), fills `animated(kind)` with the
  captured `PoolKey`s and generated `ActiveFn`s, registers
  `retarget::<H, T>` — guarded by a `HashSet<TypeId>`.
- **`elements.kind(node)`** — the node's element `TypeId`.
  `ElementTable` already tracks it for typed `get`.
- **`set_tag` re-resolve cost** — every animated field re-resolved on
  every tag event. Fine at a handful of fields; if an element ever has
  many, split `animated` back into per-tag-type buckets.
- **Despawn** — the explicit hook and the `set_tag`/`despawn` flush
  race: `transition_despawn_plan.md`.
- **Reverse easing continuity** (deferred, see above).
- **Migration.** `aim`, `aim_on`, `Transition::rebase`, the build-hook
  `.transition(field, Tween)` all go. `crates/fynix/tests/transitions.rs`
  moves to driving `set_tag` + tick.
