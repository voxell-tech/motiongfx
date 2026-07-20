# motiongfx_editor — session handoff

A working log for continuing the editor UI work. Reflects the state of
the `nixon/dock-ui` branch after a long session building the dock
system, the glass UI, and the reflect inspector.

## What this crate is

A `bevy_ui` timeline editor for `bevy_motiongfx`, on bevy **0.19**.
Modules under `src/`:

- `lib.rs` — `MotionGfxEditorPlugin` (settings/persistence) → adds
  `EditorUiPlugin` (feathers theming + scene + per-frame timeline/
  playback/preview systems). `EditorUiPlugin` lives here (not in `ui`)
  because it references the domain modules. Editor constants + `EditorState` + `EditorSettings` live here.
- `scene.rs` — component markers, the `bsn!` scene tree, `setup_editor_ui`
  (spawns UI camera, registers dock windows, seeds the `DockTree`), and
  the panel/divider drag handlers.
- `layout.rs` — `build_timeline_view` walks a track's `FragmentMeta`
  into clip/group/toggle placements and spawns them.
- `playback.rs` — play/pause (spacebar + button), scrub, playhead, time label.
- `view.rs` — retarget scene cameras to the offscreen `PreviewImage`,
  fit that image above the panel, sync name-column scroll.
- `ui/` — **UI-only, no domain deps** (future `motiongfx_editor_ui` crate):
  - `ui.rs` — misc widget builders (`themed_button`, `label`, `Divider`,
    `scrub_slider`, `playhead_line`, `clip_box`, `group_box`, `group_toggle`).
  - `ui/dock/` — the docking engine (see below).
  - `ui/glass/` — the frosted-glass material + widgets (see below).
  - `ui/inspector.rs` — generic reflect inspector.
  - `ui/theme.rs` — `EditorTheme` (Monokai Pro palette + semantic slots).

## The dock system (`ui/dock/`)

Ported from a sibling project's `jackdaw_panels`. Data/UI separation:

- `tree.rs` — pure-data `DockTree` (binary tree: `Leaf` tabbed areas /
  `Split` axis+fraction+two children). `NodeId`/`TabId` are monotonic,
  never-reused. Fully unit-tested (14 tests). `simplify()` collapses
  empty non-persistent leaves.
- `reconcile.rs` — `DockTreeHost` + `NodeBinding`; diffs the `DockTree`
  into UI entities each frame the tree changes. Leaves → `DockArea` +
  tab bar + content; splits → flex container + two panels + `PanelHandle`.
- `split.rs` — resizable `Panel`/`PanelGroup`/`PanelHandle`. Divider drag
  uses **absolute cursor position within the two panels' span** (not
  delta) so off-screen/out-of-limit drags clamp instead of banking up.
  Hover highlight is suppressed for the whole drag via `HandleDragging`.
- `tabs.rs` — tab bar spawn, tab tiles (`glass::Glass::tab(active)`),
  close buttons, hover pill swap (`hover_tabs`), and `spawn_ghost_tab`
  (the drag ghost reuses the real tab-tile builder).
- `drag.rs` — `Pointer<Drag>` state machine: reorder within a bar, merge
  into a leaf, or edge-split. `drop_on_edge` **splits before removing**
  so dropping a leaf's last tab on its own edge doesn't lose it.
- `add_popup.rs` — the "+" add-window popup (see IN PROGRESS below).
- `registry.rs` — `WindowRegistry` of `DockWindowDescriptor { id, name, build }`.

Editor integration: `setup_editor_ui` registers `viewport`, `timeline`,
`settings` windows and seeds a vertical split (viewport 0.7 / timeline).
Close guard: `handle_close_clicks` refuses to close the last tab in the
whole layout.

## The glass UI (`ui/glass/`)

Frosted-glass `UiMaterial`. **Style markers vs widgets are deliberately
separate** (user was firm on this):

- **Markers** = the `Glass` component directly: `Glass::Panel`, `Glass::Bar`,
  `Glass::Popup`, `Glass::Overlay`, `Glass::Field`, `Glass::TabActive/TabIdle/TabHover`,
  `Glass::Button`, plus `Glass::tab(active)` helper. An `attach_glass`
  observer swaps in the matching `MaterialNode`. Don't also set
  `BackgroundColor`/`BorderColor` — the material replaces both; corner
  rounding comes from the node's own `BorderRadius`.
- **Widgets** = `glass_button()` and `glass_checkbox()` and `glass_field()`
  (only these carry the `glass_` prefix). All return `impl Scene` — usable
  in `bsn!` or via `spawn_scene(...)`. `glass_button` is layout/marker-
  agnostic: append your own `Node`/`Children`/marker.
- Files: `material.rs` (GlassMaterial + builders), `preset.rs` (`Glass`
  enum, `GlassAssets`, `build_assets`, `attach_glass`), `glow.rs`
  (cursor glow + button hover/press swap), `backdrop.rs` (`GlassBackdrop`
  + `sync_backdrop`), `widget.rs` (the widget builders + checkmark/field systems).
- Shader `glass.wgsl`: SDF rounded-rect, thin near-opaque rim, frost
  (3×3 blur of a `GlassBackdrop` image — currently no backdrop is tagged,
  so frost is dormant), cursor glow gated per-material by rect containment
  (`update_glow`).

### Bloom (discussed, NOT built)
UI is composited **after** post-process (`ui_pass.after(PostProcess)`),
so a `Bloom` on the UI camera won't touch it. To bloom the glass glow:
render UI to an HDR target → bloom that, and push rim/glow **>1.0** in the
shader. Use a **threshold ≈1.0** so the LDR preview (`Rgba8UnormSrgb`,
≤1) is naturally excluded — no double-bloom of the 3D scene. Deferred.

## Theme (`ui/theme.rs`)
`EditorTheme` resource: Monokai Pro palette (mirrors
`examples/bevy_examples/assets/typst/monokai_pro.typ`) + semantic slots
(`text_primary`, `text_muted`, `accent`, `hover_fill`, `hot`). All glass
presets and dock text derive from it. Not yet a hot-reloadable asset —
that was discussed (ThemeToken vs bevy_settings resource) but deferred.

## KEY GOTCHAS (learned the hard way)

1. **Prefer `Pointer<Click>` over `Activate` for buttons.** The headless
   `Button` only fires `Activate` `if pressed` at click time — fragile.
   We migrated all buttons (play/pause, save, group toggles) to inline
   `on(|mut click: On<Pointer<Click>>, ...| { click.propagate(false); ... })`
   observers. `Pointer<Click>` bubbles, so if the handler needs the
   button's data, either **capture it in the closure** (best) or walk up
   via `ChildOf` from `click.entity`.
2. **`on(...)` in bsn** attaches an entity observer (`entity.observe`) and
   accepts closures (incl. `move` closures capturing `Clone` data) and
   named fns. Inline closures are the preferred style now (no tag
   components, no global observers).
3. **bsn field syntax / `template_value` need `Default`.** A data
   component holding an `Entity` (no `Default`) can't go in `bsn!` cleanly
   → either `.insert()` it after `spawn_scene`, or capture the data in an
   inline closure instead of a component.
4. **A plain component tuple is a `Bundle`, not a `Scene`.** bsn needs
   `Scene`; that's why widget builders return `impl Scene` via `bsn!`.
5. **No screenshots / no `cargo run`.** See the memory note — Nixon tests
   the UI; verify with `cargo check`/`cargo test` + a written checklist.
6. **Use `bevy::platform::collections::HashMap/HashSet`**, not `std`.
7. Clippy `type_complexity`/`too_many_arguments` are allowed crate-wide in
   `lib.rs` (inherent to Bevy ECS). Keep clippy otherwise clean.

## IN PROGRESS / NEXT STEPS

1. Deferred (discussed, not started): hot-reloadable theme asset; UI
   bloom (HDR target + thresholded bloom); glass over-UI refraction
   (needs two-camera grab-pass); making `PreviewImage` HDR for real
   scene highlights.

## Build / verify

```
cargo check -p motiongfx_editor
cargo test  -p motiongfx_editor      # 14 dock tree tests
cargo clippy -p motiongfx_editor     # keep at 0 warnings
cargo check -p bevy_examples --example editor --example dock_demo
```
Do NOT `cargo run` the app (per the no-screenshots memory) — hand Nixon a
manual test checklist instead.

## Manual test checklist (hand to Nixon)
- Play/Pause + spacebar toggle; Save persists `EditorSettings`.
- Drag tabs: reorder, merge into another area, edge-split; ghost matches
  the tile; edge-drop a sole tab onto its own edge doesn't vanish it.
- Divider drag clamps off-screen; highlight shows for whole drag.
- "+" opens the add-window popup; picking a window adds it; backdrop click
  closes; popup doesn't freeze the rest of the screen.
- Settings panel: glass number fields + checkbox; hover glow only on the
  hovered interactable (not neighbors).
