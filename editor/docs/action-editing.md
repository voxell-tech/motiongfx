# Action editing

Plan for the next round of timeline work: simpler block types, naming,
collapsing, and a way to actually create an action from the editor -
today nothing does; every `ActionCmd` in a scene comes from a
hand-authored file. Mock: https://claude.ai/code/artifact/c8c57aa4-80dc-47b7-9971-6d8dd9c23d5f

## Simplify `Combinator` to Chain / All / Flow

`Combinator` keeps four variants today: `Chain`, `All`, `Any`,
`Flow(Duration)`. `Any` is a genuinely different runtime primitive
(`motiongfx::track::any()` takes the *min* duration - a race, not a
wait-for-all), not foldable into anything else. `Flow`'s own duration
math already generalizes `All`'s (stagger = 0 is `All`), so the two
already sit close together conceptually, but they stay separate
variants: the action panel picks between exactly three types (a radio,
not a toggle on `All`), matching what the format already looks like
once `Any` is gone.

`Any` is real, existing asset files use it; those files are hand-edited
directly rather than adding load-time migration logic to the format
itself.

- [x] Delete `Combinator::Any`. Fixed every now-non-exhaustive match:
      `motiongfx_scene::compile.rs` (`ord_*` dispatch), `moxie/block_layout.rs`
      (label, duration, lane assignment, plus the now-dead
      `visual_end`/overlap-detection machinery `Any`'s asymmetric
      timing needed, removed along with it), `moxie/ui/action.rs`
      (`combinator_name`).
- [x] `hello_world.mox`'s one `Any` block edited to `Chain` directly.
      No other `.mox`/`.mgx.ron` asset uses it.
- [x] Action panel: replaced the read-only "Combinator" row (and the
      "Children" row next to it, dropped as clutter) with a `Type`
      row holding a 3-way segmented control (Chain / All / Flow) -
      `moxie_ui::elements::SegmentedControl`, a reusable composer, not
      wired to anything scene-specific itself. Styled on
      `ButtonElem`/`SegmentButton`, mirroring bevy_feathers'
      `RoundedCorners`/`ButtonVariant::Primary` pattern: selected
      segment filled solid, no border, 1px gap as the seam between
      segments, only the row's own two ends rounded (not themed yet -
      see `docs/backlog.md`). Picking Flow still reveals its stagger
      field the same way Delay already appears conditionally.

## Name blocks and actions

`Block` and `ActionCmd` have no name field today. Adding
`name: Option<String>` to both, `#[serde(default, skip_serializing_if
= "Option::is_none")]`, is purely additive - an old scene file just
deserializes with `None`, no migration needed (unlike `Any` above).

- [x] `name: Option<String>` on `Block` and `ActionCmd`.
- [x] Action panel: the heading itself is the editable Name field
      (reusing the registered `String` `Inspect` widget, same as any
      other text field) - no separate read-only title plus a Name row
      beneath it; clicking the heading is what a text field already
      does. Simpler than the mock's promote-on-set heading, and drops
      needing a bold/muted or italic-placeholder distinction (`Label`
      has no italic support today anyway).
- [x] Timeline: a block's header shows its name if set, falling back
      to its combinator type (`block_layout::block_label`);
      `TimelineAction` grew an optional label child, pinned
      top-left and clipped, so a bar too narrow for its name just
      shows nothing instead of overflowing its neighbor.

## Collapsible blocks

`Foldable` itself doesn't apply here - it's a vertical-list composer
(header row, then an indented body below), and the timeline is
absolutely-positioned boxes on a time axis, not a flex column. What's
reused instead is the convention: `fold::CHEVRON_SHUT`/`CHEVRON_OPEN`
rotation values and a `BTreeSet<Vec<usize>>` fold-state resource
(`timeline::BlockFoldState`), the same shape `assets::AssetFoldState`
already uses for the asset browser's own folder tree.

- [x] `BlockFoldState`, by path. `block_layout::layout` takes it and
      threads it through measurement: a folded block's children are
      skipped (empty, not built) and its height drops to just the
      header strip's - its duration is untouched, so it still
      contributes its full width/timing to siblings. Toggling is a
      plain `value_changed(block_view)` rebuild, same as any other
      timeline edit - no extra resource-changed wiring needed, since
      `Placed` already reflects the new state.
- [x] The chevron is its own absolutely-positioned element beside
      `TimelineBlock`'s header, not nested inside it - a nested
      `AlignItems::Center` row centers in the whole block's height
      (header *and* all its content), not just the header strip,
      since that's the box's real height. Independent positioning
      sidesteps that entirely.

## Unassigned actions (`Node::Draft`)

A third `Node` variant, not `Option` fields on `ActionCmd` - keeping
`ActionCmd` fully required everywhere it's already assumed to be
(compile.rs, the registry's typed resolvers, `subject_of`/`field_name`)
and putting "not yet specified" in the tree shape instead.

```rust
Node::Draft {
    delay: Option<Duration>,
    duration: Duration,
    name: Option<String>,
}
```

Living in the tree from creation means every mechanic already built
around "a `Node` at a `path`" - selection, drag-to-merge/chain/out,
naming, a parent block's collapse - applies to a draft with no extra
plumbing. The alternative (an editor-only staging list outside
`scene.0.animation`) would need its own parallel selection, placement,
and timing concept instead of reusing what's there.

- [x] `Node::Draft` variant. New match arm everywhere `Node` is
      matched exhaustively: `walk_node` (compile.rs), `measure_node`
      (block_layout.rs, plus `node_duration`), `summarize`/
      `Property::get`/`set`/`seconds`/`set_seconds` (action.rs).
      `node_at`/`node_at_mut` needed no change - they already fall
      through on anything that isn't `Node::Block`.
- [x] `compile()`: a draft resolves to `TrackFragment::silent(duration)`
      (new, alongside `TrackFragment::new`/`single`) - a fragment with
      no clips that still reserves its timing slot. Not a
      `CompileError`; being incomplete isn't being broken.
- [x] Action panel: `draft_shape` - Duration/Delay/Name are real edits
      (the same `Property` machinery Action/Block already use);
      Subject/Field are still plain "Unassigned" text for now. The
      picker UI itself is its own follow-up, not built this round.
- [x] Timeline: a draft's clip reads as an empty slot (a faint outline,
      no fill, "Draft" label) rather than a real action's solid fill -
      `Placed` grew a `draft: bool`.

### Graduating: draft to action

Not an ECS hook - `Node::Draft` lives in plain scene data
(`EditorScene`), not entities/components, so there's nothing for a
Bevy observer to react to. It's a plain check in the same write-back
path `Property::set` already uses: whenever a picker writes subject or
field, check whether *both* are now set, and if so, replace the
`Node::Draft` at that path with a real `Node::Action` via
`node_at_mut` (same helper `Property::set` already calls).

- [ ] `op` defaults to `AnimOp::To` (the only variant that exists
      today).
- [ ] `value` (the *target*, not a starting point - `AnimOp::To`'s own
      op closure ignores `_prev`) defaults to the field's current live
      value, captured through the same reflect read `Property`/`Field`
      already do. Allocate a new pool `Uuid`, insert into
      `scene.0.values`'s matching typed column.
- [ ] Open question: fire automatically the instant both are set, or
      require an explicit confirm? Field-picking is a multi-click
      drill into a reflected type tree, so an intermediate click
      landing on a field before the user's done browsing could
      prematurely graduate and capture the wrong value. Superseded in
      practice by drag-to-create below, where subject+field arrive
      together in one gesture - worth revisiting whether this path is
      still needed at all once that ships.

### Demoting: action back to draft

The reverse direction. `CompileError::UnknownSubject` already exists
because today, deleting an entity an action targets hard-fails
`compile()` for the *whole scene* - `walk_block` unwinds through `?`
at every level, no per-action recovery, and `recompile_dirty_scene`
calls `.compile()` with `.expect(...)`, so this currently panics the
editor, not just fails cleanly. Demoting the orphaned action to a
draft instead turns "the whole scene stops compiling" into "one action
needs reassignment."

Two different triggers, two different demotion shapes - plus a third,
generic one that landed first, as a safety net rather than a proactive
fix:

- [x] `recompile_dirty_scene`: a `CompileError` no longer panics the
      editor. `demote_offending_action` walks the tree for whichever
      `Node::Action` the error names (by subject/field/op/value/ease/
      interp, matching the error variant), demotes just that one, and
      retries - looping until it compiles or nothing more can be
      demoted. Catches anything unresolvable regardless of cause (a
      hand-edited scene file, a typo'd field path), not only the two
      triggers below - but reactive, not proactive: it only runs the
      next time something recompiles, not the instant an entity dies.
- [ ] Entity deleted: subject itself is gone, so nothing about the
      action is still valid. Extend the existing `on_remove_entity_uid`
      observer (`bevy_motiongfx/scene/id.rs`, already keeps
      `SceneUidMap` in sync on `On<Remove, EntityUid>`) to also walk
      `scene.0.animation` and demote every `Node::Action` whose
      subject matches - full demotion, clear subject/field/op/value,
      keep duration and any name.
- [ ] Component removed, entity still alive: only the field is
      dangling. Partial demotion - keep subject, clear field/op/value,
      so the user only re-picks a field on the same entity. No single
      generic "a component was removed" event exists in Bevy for
      arbitrary types; this wants a validation pass over the tree's
      real actions (resolve each `FieldRef`'s type-name to a `TypeId`
      via `AppTypeRegistry::get_with_type_path`, same lookup
      `field_name` already does, and check the subject still has it)
      run when relevant things change, not one elegant observer.
- [ ] There's no undo system in this editor at all. Auto-demoting on
      delete is a silent, non-reversible rewrite of every action
      touching that entity. Worth a confirmation prompt on deletion
      when it would orphan live actions, even without full undo.

## Moving and resizing in the timeline

There's no stored "start time" anywhere in the data model - every
position in `block_layout.rs` is computed from the tree (combinator +
duration of preceding siblings + `delay`). The only knob that actually
exists to drag is `delay`, already the existing Delay edit row, and
what dragging means with it depends on the parent:

- Inside a `Chain`, siblings are already sequential - giving one a
  `delay` only opens a gap before it, it can't move past a neighbor.
  Dragging past a sibling here means reordering `children`, not tuning
  `delay`. Exact precedent already in the codebase:
  `hierarchy/drag.rs`'s before/after drop-target pattern (used for
  reparenting subjects) is the same "which list, and where in it"
  problem, just applied to `Block.children`.
- Inside `All`/`Flow`, or a block's own position in its parent,
  dragging maps directly onto editing `delay`.

Duration is asymmetric between actions and blocks:

- An action's `duration` is a plain stored field, already editable via
  the panel's Duration row. Dragging its right edge to resize is the
  same edit, just via drag instead of typing a number.
- A block's duration isn't stored at all - `block_duration()` derives
  it from its children. A block has no edge to resize; only its body
  is draggable (a move, via `delay`, same as any other node). Resizing
  a block would mean time-stretching every child's duration/delay
  proportionally - not in scope.

Unifies with the merge/chain/drag-out design already mocked, rather
than adding a fourth separate mechanic: drag an action's edge = resize
(the only edge-drag there is); drag any node's body and release in
open space = move (`delay` edit, or a `children` reorder inside a
`Chain`); drag a body and release on/near another action = merge or
chain, as already designed. Same `Dragging`-style resource, same
ghost-preview visuals, one gesture vocabulary.

- [x] Edge-drag resize - `TimelineBlock` gets no resize handle,
      body-drag only; `TimelineAction` gets one, and so does a draft's
      empty-slot box (it has a `duration` too). A small `Frame`
      overlaid on the box's right edge, same trick as the block
      header's chevron - not nested in `TimelineAction` itself, which
      has no room to distinguish an edge from its body.
- [x] Body-drag move, both `TimelineAction` and `TimelineBlock`
      (`timeline/drag.rs`): edits `delay` live inside `All`/`Flow`/at
      a block's own position; reorders `children` inside a `Chain`.
      The reorder is simpler than `hierarchy/drag.rs`'s live
      before/after drop-target highlighting - no ghost, nothing
      redraws until the drop, when the target index comes from the
      drag's total distance against where its siblings started (each
      one's own `Placed.x`/`.w`, snapshotted at drag-start). Good
      enough to reorder correctly; a live indicator while dragging is
      still open.
- [ ] Not built: the merge/chain/drag-out restructuring the intro
      above alludes to (drop onto another action to group them into a
      new `All`/`Chain`) - out of scope for this round, its own
      follow-up.

## Creating an action: drag a field onto the timeline

Better than draft-then-assign for the common case: drag a field row
straight out of the Component Inspector onto the timeline. Subject
(whatever entity is inspected) and field (the row dragged) arrive
together in one gesture - this *is* the graduation moment, condensed,
so for this path there's no draft phase at all. `Node::Draft` becomes
purely what demotion produces, not something manually created for
this flow; a manual "blank slot" affordance (block out timing before
deciding what to animate) may still be worth keeping alongside this -
open question.

- [ ] Field eligibility (animatable, not just inspectable) decided at
      row-*build* time, not drag-time: an ineligible field gets no
      drag handle at all, not a refusal on drop. `moxie_ui`'s
      `EntityInspector`/field-row builder has never known about
      `SceneRegistry` and shouldn't start now (same boundary as the
      rest of this session's reuse work) - the eligibility check and
      the drag-start wiring both live in `moxie`, layered onto the
      field rows `EntityInspector` already builds, not inside
      `moxie_ui`'s generic `Inspect` widget system.
- [ ] `SceneRegistry` needs a new read method - it currently only has
      `register_*`, no way to ask "is this `FieldRef` registered."
- [ ] Per-axis dragging for vector fields. `inspector/vector.rs`'s
      `axes()` renders x/y/z as one combined `Inspect` leaf today, on
      purpose, for editing ("the widget needs no way to address an
      axis on its own"). The scene format already supports a sub-path
      like `"translation::x"` (see `refs.rs`, `roundtrip.rs`) - the
      gap is only the widget. Give each of the three number inputs
      its own drag-start handle, composing the base path with
      `::x`/`::y`/`::z`.
- [ ] Cross-panel drag reuses the pattern `hierarchy/drag.rs` already
      establishes: a `Resource` tracking what's held and where it'd
      land, driven by Bevy's own `Pointer<DragStart/Drag/DragOver/
      DragDrop/DragEnd>` events, which fire on whatever's under the
      cursor regardless of which panel it's in. Needs a parallel
      resource (or a generalized one) with drop-target observers added
      to the timeline's scene area.
- [ ] Live ghost during the drag: reuses the exact visuals already
      built for action-to-action dragging (dashed accent border,
      reduced opacity) rather than new vocabulary. The timeline's
      `DragOver` handler computes the prospective start time from the
      pointer's x (inverse of `px_for`) and draws the ghost there,
      sized to a default duration.
- [ ] Dropping onto/near an existing action should trigger the same
      merge-into-All / snap-into-Chain restructuring already designed
      for action-to-action dragging, rather than a separate drop path.
      Open question: should a drop also be allowed to land anywhere on
      empty timeline space, or only inside an existing block / on an
      existing action?
- [ ] On drop: `op` defaults to `AnimOp::To`, `value` captured from
      the field's live value at drag-start (same capture as
      graduation), duration defaults to some fixed constant until
      there's a reason to make it drag-configurable.

## Editing the stage

The stage (`Scene::stage`, `Subject.fields: Vec<FieldSeed>`) is not an
action's own value - `AnimOp::To`'s op closure ignores `_prev`
entirely, so an action only ever carries its *target*. The stage is
specifically the starting point for the *first* action on a field with
no predecessor; later actions in a chain inherit the previous action's
target instead. Nothing edits the stage today - grepped the whole
editor, `Stage`/`FieldSeed` are read-only, at one `.stage()` call site.

- [ ] A toggle on the Component Inspector's own field rows, to pin the
      field's current live value as its stage seed. Doesn't require an
      action to exist first, which matters - a field can want a
      specific starting pose before anything animates it yet.
- [ ] A companion row in the action panel, shown only when the
      selected action is the earliest one touching that subject+field
      (needs `summarize()` to find that by walking the tree in time
      order) - so editing an action's timing doesn't require leaving
      the panel to see what it starts from.

## Explicitly backlogged, not part of this round

- [ ] Incremental names for newly created entities ("Cube", "Cube 1",
      "Cube 2", ...).
- [ ] The Operation row: `AnimOp` has exactly one variant (`To`) today,
      so it stays a plain hidden/read-only row rather than a variant
      picker (the same `VariantPicker` pattern Ease/Interp already
      use) until a second op actually exists.
