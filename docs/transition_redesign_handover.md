# Transition redesign: a CSS-style push model

Handover notes from a design conversation. Nothing here is built. It
supersedes the `aim` / `aim_on` model, not the storage that shipped
first: `TransitionTable<H>` over `typarena::TypeTable`, one
`Transition<H, T>` column per value type, keyed by `FieldKey<H>`. That
storage stays; `Slot` / `Run` below replace what fills the columns.

## Where the current model falls short

`ElementMut::transition` declares a `Transition<H, T>` per field.
`Fynix::aim(node, field, Some(x) | None)` points it; a backend wires
`aim_on(Enter, ..)` / `aim_on(Leave, ..)` over its own events. The
`Transition` holds `base`, `target`, `from`, `elapsed`.

Problems:

- **`base` is a copy kept fresh by push.** A binding on the same field
  calls `Transition::rebase`. Forget that call and a release animates
  to a stale value.
- **The target is captured at build.** `aim_on(Enter, .., Some(hover))`
  freezes `hover`; if the element's hover value changes at runtime, the
  transition can't see it.
- **`advance` iterates every field with a transition every frame**,
  settled or not (each early-returns, but still).
- **Setup is spread across three places** that must agree: the field's
  `#[elem(patch = ...)]`, the `.transition(..)` call in the build hook,
  and the duration passed there rather than at the field.

## The CSS model (the reference)

- **Computed style** = the cascade plus pseudo-classes (`:hover`,
  `:focus`, ...). This is the *target*. Recomputed on **style recalc**,
  which the browser triggers on a discrete change (class toggled,
  `:hover` flipped, a custom property reassigned).
- **`transition:`** is metadata on the element: "if this property's
  computed value changes, interpolate instead of snapping."
- **The transition engine** is separate, keyed by *(element,
  property)*. Each running transition stores a fixed start, fixed end,
  timing, start time. Painting reads the interpolated value; the
  computed style still holds the target underneath.
- **The per-frame tick is pure interpolation of a frozen start -> end.**
  It never re-diffs. The target is resolved once, at recalc.
- **On the next recalc** the engine diffs the new computed value
  against what is running: different end -> cancel, start a new
  transition **from the current displayed value** (no pop); end equals
  the reversing-adjusted start -> treat as a reverse and shorten the
  duration proportionally to how far it got.

## Proposed shape

### `Slot` (persistent) + `Run` (transient)

```rust
// one per animated field, in the per-T TypeTable column
struct Slot<H: Host, T> {
    write:   fn(&mut Patch<H>, &T),          // from the field's `patch =`
    resolve: fn(&ElementTable<H>, &H::World, H::Node) -> Option<T>,
    curve:   Tween<T>,                       // Duration + ease + lerp
    shown:   T,                              // last value written; next Run's start
    run:     Option<Run<T>>,
}

struct Run<T> {
    from: T,
    to:   T,        // frozen until a recalc changes it
    elapsed: Duration,
}
```

`Slot` lives for the node's life. `run: None` when settled: ~2 ptrs +
a `Tween` + one `T` + a discriminant, inline in the column, no box.
This is CSS's split exactly - `Slot` is the `transition:` rule, `Run`
is the running transition.

### The resolver reads the world like a binding does

No `Host::Interaction` associated type - one fixed struct would be a
god-object (a button wants hover/press, a text field wants
focus/selection, a third-party element wants its own component). The
resolver gets a `WorldNodeRef<H>`, the same handle `bind`'s value
closure gets, and reads whatever it wants:

```rust
fn bg_target(&self, w: WorldNodeRef<FynixHost>) -> Color {
    if w.pressed()      { self.active_bg }
    else if w.hovered() { self.hover_bg }
    else                { self.bg }
}
```

`w.hovered()` / `w.pressed()` / `w.get::<MyToggleState>()` are
backend-provided accessors. "Interaction state" stops being a fynix
concept - it is world data the resolver chooses to read, exactly like
app state in a binding.

### Push, not pull: recalc on discrete events

The per-frame tick is pure interpolation of `Run`'s frozen
`from -> to`. Targets change only at **recalc**, triggered by:

- a binding fired for node N, or
- N's interaction state changed (bevy picking is evented -
  `Pointer<Over>` etc. tell fynix).

On either, re-resolve **all of N's slots** (coarse, like a browser
recalcs every property of the affected element; few slots per node, so
no per-field dependency graph to get wrong).

```rust
fn recalc_slot(slot, elements, world, node) {
    let Some(target) = (slot.resolve)(elements, world, node) else { return };
    match &mut slot.run {
        None if slot.shown != target =>
            slot.run = Some(Run { from: slot.shown.clone(), to: target, elapsed: ZERO }),
        Some(run) if run.to != target => {
            let here = lerp(run, &slot.curve);
            *run = if is_reverse(run, &target) {
                Run { from: here, to: target, elapsed: reversed(run, &slot.curve) }
            } else {
                Run { from: here, to: target, elapsed: ZERO }   // start from here, no pop
            };
        }
        _ => {}
    }
}

fn tick_slot(slot, dt, world, node, theme) {
    let Some(run) = &mut slot.run else { return };
    run.elapsed += dt;
    let t = slot.curve.at(run.elapsed);
    slot.shown = (slot.curve.lerp)(&run.from, &run.to, t);
    (slot.write)(&mut Patch::new(world, node, theme), &slot.shown);
    if run.elapsed >= slot.curve.duration {
        slot.shown = run.to.clone();
        slot.run = None;                 // Run gone, Slot stays
    }
}
```

Fallback if the backend cannot report interaction changes: poll
`resolve` once per flush for every slot (N animated fields, an extra
world read each - same order as the bindings loop). An optional
`Host::interaction_generation(node) -> u64` lets a poll skip unchanged
nodes.

### Rules

- **`anim` on a field => the animation is the sole writer.** `patch = X`
  supplies the how-to-write fn; `bind` on that field only does
  `*field = new`. No competing writers, so dropping `Run` on settle is
  safe.
- **Interruption starts from `shown`.** Reverse detection shortens the
  duration; everything else restarts the leg from the current value.
- **`Duration`, not `f32`.** `Tween.duration` already has the TODO.

### Field taxonomy

`#[element]` needs a fourth kind of own field: addressable, no writer.

| form | backend writer | addressable (`bind`) | meaning |
| --- | --- | --- | --- |
| `#[elem(patch = X)]` | yes | yes | a drawn property |
| bare (no `#[elem]`) | no | yes | element state: feeds resolvers, not drawn |
| `#[elem(ignore)]` | no | no | build-time scratch only |
| `#[elem(child)]` | - | - | sub-element |

`bind` today requires `P: Bindable<H>` (the field has a `FieldPatch`
tag). It must accept bare fields: fire path becomes `*field = new;
if P is Bindable { patch(..) }`. Then `hover_bg` and friends are
runtime-updatable, not frozen at build - a theme change binds them and
mid-hover buttons ease to the new value on the next recalc.

### Macro wiring

Everything at the field except the resolver body. No ZST tag - the
resolver is a method path (a bare fn, usable as a generic arg;
closures would need boxing).

```rust
#[element]
pub struct ButtonElem {
    #[elem(patch = PatchBackground, anim(ms = 120, ease = quad::ease_out, via = Self::bg_target))]
    pub bg: Color,
    pub hover_bg: Color,     // bare: bindable, read by bg_target
    pub active_bg: Color,
}

impl ButtonElem {
    fn bg_target(&self, w: WorldNodeRef<FynixHost>) -> Color { /* 3-line match */ }
}
```

The macro emits, in `build()` beside the field write:

```rust
slots.register::<Self>(owner, bg_marker::id(), PatchBackground::patch,
                       target_of::<H, Self, { Self::bg_target }>, CURVE);
```

No `via` => identity resolver (`|e, _| e.field`) and a default curve:
"ease toward my own value on change", the current `transition()` use,
still declarative.

## Open questions

- **Separate crate?** The instinct is right (continuous/time-driven vs.
  discrete/event-driven), but it is welded to `ElementTable`,
  `FieldKey`, `Patch`, `Store::resolve`, the flush loop, and
  `#[element]` codegen. Recommendation: do it as `fynix::anim` /
  `fynix::transition`, one flush entry point
  (`slots.recalc_and_tick(elements, world, theme)`) and one stable
  trait the macro emits through. Extract later if the boundary proves
  clean. `motiongfx_interp` is already a dep.
- **Reverse detection.** `is_reverse` / `reversed` need spelling out -
  CSS's reversing-adjusted-start rule, or a simpler "if `to` is the
  previous `from`, scale `elapsed` by progress".
- **`start` re-snapshot trigger.** Cached-`to` compare in `recalc`
  (needs `T: PartialEq`) vs. a poke from the interaction system.
- **Drop-on-arrival for `Slot` itself.** Currently `Slot` stays for the
  node's life. Lazy seeding (materialize `run` + `shown` only while
  moving, keep just the recipe otherwise) is the further step if the
  resident cost matters.
- **Poll vs. evented interaction.** Whether `bevy_fynix` wires picking
  events into a `recalc(node)` call, or fynix polls every slot each
  flush.

## Migration

Replaces `Fynix::aim`, `aim_on`, `Transition::rebase`, and the
build-hook `.transition(field, Tween)` call. `Tween<T>` (the spec)
stays. `TransitionTable` becomes a `SlotTable` (or keeps the name).
The `crates/fynix/tests/transitions.rs` suite is written against
`aim` and would move to driving `WorldNodeRef` state + recalc.

## Left over from the storage rewrite

- **Delete `Element::patch`.** Nothing outside the generated code calls
  it any more - `bind` and `transition` both go straight through the
  tag. Still exercised directly by `crates/fynix/tests/`
  (`elements.rs`, `generics.rs`, `siblings.rs`), which would move onto
  the tag path or be dropped. `build`'s field writes and `despawn`
  stay; only `patch` and its `own_patches` / `child_patches` codegen
  go.
- **Bench** a splitter drag and a playing timeline, before and after
  the column storage, to put a number on it.
