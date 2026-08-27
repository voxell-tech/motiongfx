# Backlog

## Unify theming onto `EditorTheme`

`EditorTheme` (our Monokai Pro palette) and feathers' `UiTheme`
currently coexist. `UiTheme` only colors the stock feathers widgets we
reuse - `NumberField`'s input and the dropdown popup - which likely
look mismatched against the rest of the UI. Their own systems read
`UiTheme` every frame for hover/press/focus, so patching tokens still
leaves a second theme mechanism underneath; owning the widgets is the
only way both surfaces read one palette. Fork feathers and rebuild
those as moxie_ui elements styled off `EditorTheme`, then drop
`UiTheme`/`ThemeProps`/feathers tokens once nothing reads them.

- [ ] Fork feathers; build our own `NumberField` (drag/type/format
      interaction) as a moxie_ui element, styled off `EditorTheme`.
- [ ] Build our own dropdown popup (placement, dismissal, keyboard
      nav) as a moxie_ui element, styled off `EditorTheme`.
- [ ] Drop `UiTheme`/`ThemeProps`/feathers tokens once nothing reads
      them.
- [ ] `Label`'s `None => ThemedText` fallback should default to
      `theme.text_primary` directly.
- [ ] Give `EditorTheme` a button corner radius (there's no field for
      one today) and use it everywhere a button rounds itself -
      `Button`/`GhostButton`/`MenuButton`/`SegmentButton` each pick
      their own `px(N)` constant right now.

## Unify icon size across icon-only buttons

Every icon-only button (a fold chevron, a variant picker's, a
toolbar-style button) currently sets its own `size` on the `Icon` it
carries, and they disagree: `fold::Foldable` uses `px(8)`, `inspector/
enums.rs`'s `VariantPicker` chevron `px(9)`, `ui/timeline.rs`'s block
chevron `px(4)`, and `ui/assets.rs`'s two folder-row chevrons plus
`inspector/tree.rs`'s leave it unset, falling through to `Icon`'s own
default of `px(11)`.

A style can't fix this from `ButtonElem`'s side: fynix_mock's cascade
is element `Default` -> `Style` -> call site (`style.rs`), and the
call site's own `icon = val!(Icon, ...)` assignment replaces the whole
field, style-applied or not - a `Style for ButtonElem` (`TintButton`
and friends) runs before that and gets overwritten regardless. The fix
has to be a `Style for Icon` instead, used at the icon assignment
itself: `icon = val!(!SomeIconStyle, image = ..., color = ...,
rotation = ...)`, dropping each call site's own `size = px(N)`. `val!`
already supports the same `!style` form `elem!` does.

- [ ] Add a `Style for Icon` in `moxie_ui/elements` fixing `size` to
      one constant.
- [ ] Convert every icon-only button's `icon = val!(Icon, ...)` to the
      new styled form, across `fold.rs`, `inspector/enums.rs`,
      `ui/timeline.rs`, `ui/assets.rs` (two spots), and
      `inspector/tree.rs`.
- [ ] Pick the actual size - `px(8)` (`Foldable`'s current value) is
      the closest thing to an existing default among them.

## Open a `.mox` by double-clicking it

Needs moxie shipped as an installed app: the OS only routes a document
to something it has a registration for. Windows and Linux hand the path
over as `argv[1]`, so reading it is `env::args_os().nth(1)` either way;
registration is a `HKCU\Software\Classes` key on one and a `.desktop`
plus a shared-mime-info XML on the other.

macOS is the awkward one. Finder never passes the path in `argv` - it
sends a `kAEOpenDocuments` Apple Event, and only to a real `.app`
bundle. Winit's macOS docs say it "guarantees that it will not register
an application delegate" and show an `application:openURLs:` example,
but that is wrong for 0.30: it registers `WinitApplicationDelegate` in
`EventLoop::new`, and its run loop asserts every iteration that the
delegate is still that type, so setting our own panics on the first
turn. The way in is `NSAppleEventManager`'s
`setEventHandler:andSelector:forEventClass:andEventID:`, which leaves
`NSApp.delegate` alone - registered after `EventLoop::new` but before
the app runs, or the launching document is missed. `objc2-app-kit` and
`objc2-foundation` are already in the tree, but on two major versions
at once: winit is on the 0.5/0.2 generation, and types do not cross.
`setEventHandler:` has no binding, so it needs a raw `msg_send!`.

Two things break outside `cargo run` whatever we do, since both resolve
a path that only exists on the build machine: `AssetPlugin::file_path`
is `"../assets"`, relative to a working directory a launched app does
not have, and `project.rs`'s file dialog starts at
`env!("CARGO_MANIFEST_DIR")`.

Dragging a `.mox` onto the running window is a separate path that
already works - bevy maps it to `FileDragAndDrop::DroppedFile` - and
would be worth wiring up on its own.

- [ ] Take a path at startup from `argv`, and open it through a
      path-taking half of `project::load_scene` split out from the
      dialog.
- [ ] Resolve assets and the dialog's starting folder against the
      running executable rather than the build machine.
- [ ] Handle `FileDragAndDrop::DroppedFile`.
- [ ] macOS: bundle, its `Info.plist` document-type declarations, and
      the Apple Event handler.
- [ ] Windows and Linux: register the type, and decide whether that is
      an installer's job or something moxie does for itself on first
      run.

## "Open" dialog never remembers the last-used location

Reported: "Open" should start wherever a project was last opened or
saved from, but doesn't - not a case of another picker's state leaking
in, there's simply no last-used-location memory at all today.

`project.rs`'s `ask_for_path` unconditionally calls
`.set_directory(&scenes)`, `scenes` being the fixed
`CARGO_MANIFEST_DIR/../assets/scenes` - every Open and every Save
starts there, every time, regardless of what was picked last (this
session or a previous one). `ProjectPath` (already updated by both
`save_scene` and `load_scene` on success) is the obvious source of
truth for "last used" - `ask_for_path` just doesn't read it, and isn't
even passed `world` today to be able to.

- [ ] Thread `world: &World` (or just the resolved `Option<PathBuf>`)
      into `ask_for_path`, and seed `.set_directory(...)` from
      `ProjectPath.0`'s parent when set, falling back to `scenes` only
      when nothing has been opened or saved yet.
- [ ] Decide whether Open and Save should track the same "last
      location", or diverge (e.g. Save defaulting to the current
      project's own folder, Open to wherever was last opened).
- [ ] Persisting this across app restarts (not just within one running
      session) would need somewhere durable to keep it -
      `EditorSettings` is the existing precedent for that.

## `BevyElementVisual` boilerplate

Every `ElementVisual<BevyHost>` impl starts with
`world.entity_mut(node)`. A forwarding blanket impl hits Rust's
orphan rule; the marker-param workaround compiles but can't satisfy
`Element<H>`'s bound without changing `fynix_mock`'s kernel.

- [ ] Add a `macro_rules!` in `bevy_fynix` forwarding a narrower
      `BevyElementVisual` impl into `ElementVisual<BevyHost>`.
- [ ] Or an attribute macro on the impl itself, skipping the repeated
      type name - needs a new `bevy_fynix_macros` proc-macro crate.

## `WorldNode`/`WorldNodeMut` for the `(world, node)` pair

Bindings, a watcher's `ChangedFn`, and `Lane::advance` all take
`(&H::World, H::Node)` or `(&mut H::World, H::Node)` as two separate
params - the same pair `Build`/`Patch`/`ElementMut` already put behind
`EntityExt`'s `id()`/`world_mut()`. A generic `WorldNode<'w, H: Host>`/
`WorldNodeMut<'w, H: Host>` in `fynix_mock` would give bindings and
watchers one param instead of two, and `bevy_fynix` could implement
`EntityExt` for the mutable one directly, letting a free function that
only mutates its own node (or reaches one child) drop its manual
`world.entity_mut(node)` the same way `Build`/`Patch`/`ElementMut`
already did.

Only worth it for the mutating case - a function reading other
resources or entities with `node` as just an index (`text_color`,
`highlight`) doesn't want the bundle; Bevy's own `EntityRef`/
`world.entity(node)` already covers a plain read.

- [ ] `WorldNode<'w, H: Host>`/`WorldNodeMut<'w, H: Host>` in
      `fynix_mock`, holding `world` and `node`.
- [ ] `impl EntityExt for WorldNodeMut<'_, BevyHost<Theme>>` in
      `bevy_fynix`.
- [ ] Convert the `bevy_fynix`/editor free functions whose whole job is
      "mutate this node, maybe touch one child" - audit each first,
      since some `(&World, Node)` sites read other state through
      `node` as an index and shouldn't move.

## `#[elem(flatten)]` for shared field groups

`ButtonElem`, `Frame`, `ScrollArea`, `Panel`, and a few others each
hand-roll the same `width`/`height`/`min_width`/`min_height`/
`flex_grow`/`justify`/`padding` cluster and their own `node()`
builder. Embedding `bevy::Node` itself was considered and turned
down: `#[derive(Element)]` mints one enum variant, one `FieldId`, and
one `lenz` accessor path per top-level field, which is what lets
`elem!(Frame, width = ...)` work as flat sugar, `.bind(|frame|
frame.width())` name a single projection, and `#[default(...)]`
override one field at a time. Collapsing eight fields into a single
`pub node: Node` would collapse all of that onto one opaque path, and
still wouldn't absorb `radius`/`fill`/`hover`, which aren't part of
`Node` at all.

A flattened field group keeps the addressability: a small reusable
struct (`Layout { width, height, min_width, min_height, flex_grow,
justify, padding, ... }`) that `#[derive(Element)]` unpacks field by
field into the same enum/path/lenz machinery it already generates for
a direct field, rather than treating it as one opaque child.

- [ ] Design `Layout` (or similarly-scoped groups) as a plain struct
      other `Element`s embed.
- [ ] Teach `fynix_mock_macros/src/element.rs` an `#[elem(flatten)]`
      directive: for a flattened field, walk its own fields the same
      way as a direct one instead of minting a single variant for the
      whole struct.
- [ ] Convert `ButtonElem`, `Frame`, `ScrollArea`, `Panel`, and
      `field.rs`/`dropdown.rs`'s elements over once flattening lands.

## Assigning and treating assets in the inspector

`Handle<T>` fields (`MeshMaterial3d<StandardMaterial>`, `Mesh3d`, and
anything else asset-bearing) fall through the reflect-tree inspector
today as an opaque struct - there's no UI to assign one at all.
`project.rs`'s `.mox` save/load already round-trips a `Handle<T>` as a
path string, through bevy's own `world_serialization` (`WorldDeserializer`
+ `LoadFromPath`) - that's inherited plumbing, not something moxie
built, and `Mesh3d`/`MeshMaterial3d<StandardMaterial>` are already
allowlisted in `subject_components()` to go through it.
`moxie_asset`'s `StdMaterialAssetLoader` (the `.mat` loader) is the
existing precedent for a project-authored asset: a `StandardMaterial`
reflected to RON, loaded through the normal `AssetServer`/`Handle`
path like anything else - it just has no caller today besides a
one-off codegen example.

Modeled on Unity/Unreal rather than Blender/After Effects: one asset
root per project (a folder next to, or named by, the `.mox`), external
files imported into it on assignment rather than referenced in place
from anywhere on disk. This sidesteps the path-root fragility already
flagged above (`AssetPlugin::file_path` hardcoded to `"../assets"`,
broken outside a dev build) without needing a GUID/redirector system -
still plain path references, so a rename done outside the editor can
still break one, same limitation Blender/AE's relative-path model has.

Small project-authored assets (a material) get a second option:
`mox://` as a scheme on the same `Handle<T>` path, backed by
`bevy_asset::io::memory::{Dir, MemoryAssetReader}` (already built into
bevy, not hand-rolled) so they travel embedded inside the `.mox` RON
itself rather than as a separate file - closer to Lottie's per-asset
embed-or-reference flag, or Rive's embed-by-default. Not Bevy's own
`embedded://` - that id already names the compile-time
`embedded_asset!` source; ours needs a different scheme.

- [ ] Register the `mox://` [`AssetSource`](bevy_asset::io::source::AssetSourceBuilder)
      before `DefaultPlugins` (asset sources build when `AssetPlugin`
      does, not after) - one `Dir` for the process's lifetime, reused
      across project loads, not rebuilt per load.
- [ ] Stage a `.mox`'s embedded section into that `Dir` before
      `WorldDeserializer` resolves any `mox://` handle against it -
      resolution just reads whatever's there.
- [ ] A project asset folder as the one root for imported/external
      files; import (copy in) on assignment rather than reference in
      place.
- [ ] Wire `StdMaterialAssetLoader`'s serializer to an actual "save
      this material as a new asset" action - today only
      `moxie_asset/examples/gen_default_material.rs` ever writes a
      `.mat`.
- [ ] A preview panel beside the asset browser's own listing, showing
      whatever asset is currently selected or hovered there.

## Treat multi-item files (`.glb` and similar) as folders in the asset panel

The asset panel (`editor/moxie/src/ui/assets.rs`) already has a folder
concept, but only for real filesystem directories: `FolderRow` /
`BookmarkRow` both lean on `moxie_ui::fold::Foldable` and a live
`fs::read_dir` (`build_children`) to expand a directory into its
children, tracked open/closed by `AssetFoldState`. What decides
whether a *file* is even recognized is `moxie_asset::AssetKinds`, a
flat `extension -> TypeId` map with exactly one registration today
(`.mat` -> `StandardMaterial`, in `inspector.rs`); nothing maps `.glb`/
`.gltf`, so such a file currently renders inert and undraggable.

A `.glb` isn't a single asset the way a `.mat` is - it's a small
archive (meshes, materials, lights, cameras, an implicit scene graph).
Making it *browsable* as a folder means the row needs to expand into a
synthetic child list the same way `FolderRow` does, but sourced from
gltf-parsed contents instead of `fs::read_dir`. That's a second, virtual
kind of expandable row alongside the filesystem-backed one -
`FolderRow`'s body-building step (`build_children`) would need a
non-filesystem counterpart that inspects a `Gltf`/`GltfAssetLabel`'s
node list instead of walking a path, while still fitting the same
`Foldable` shell and `AssetFoldState` bookkeeping. No `Gltf`/
`GltfAssetLabel`/`SceneRoot`-from-`bevy::scene` usage exists anywhere
in `editor/` yet - this is greenfield relative to the current asset
loading code. Worth noting: the editor already has its own unrelated
`SceneRoot` marker (`editor/moxie/src/lib.rs`) naming the scene tree's
own root entity - a real gltf `SceneRoot` component would need a
distinguishing name or an explicit path (`bevy::scene::SceneRoot`)
wherever both are in scope.

Each child item (a mesh, a material, a light) still wants its own
`AssetKinds` entry so it can be dragged into a `Handle<T>` field the
same way a `.mat` can be today - a gltf-derived material handle isn't
structurally different from a hand-authored one once loaded.

- [ ] Register `.glb`/`.gltf` extensions against `bevy_gltf::Gltf` (or
      per-sub-asset types) in `AssetKinds`, so the file stops being
      inert in the browser.
- [ ] A "virtual folder" row variant that expands via a gltf's parsed
      node/mesh/material list instead of `fs::read_dir`, reusing
      `Foldable`/`AssetFoldState` rather than forking them.
- [ ] Decide the child-item addressing scheme (bevy's own
      `GltfAssetLabel` path syntax, e.g. `Mesh0/Primitive0`, is the
      obvious fit) so a child row's path round-trips through
      `AssetServer::load`.
- [ ] Wire each recognized child kind (`Handle<Mesh>`,
      `Handle<StandardMaterial>`, lights) into `draggable(...)` the
      same way `file_row` already does for flat files.
- [ ] Generalize past `.glb` specifically once the pattern holds -
      any future container format (e.g. a multi-clip animation file)
      wants the same virtual-folder shell, not a bespoke one per format.

## Drag a `.glb` straight into the hierarchy to spawn its scene

Two unrelated drag systems exist today, and neither reaches across to
the other. `editor/moxie/src/ui/hierarchy/drag.rs`'s `Dragging`
resource only reacts to other hierarchy rows (reparent/reorder within
the tree). `editor/moxie_ui/src/asset.rs`'s `AssetDragging` resource -
structurally the same ghost-follows-cursor pattern, for files instead
of rows - is only ever *read* by one consumer:
`editor/moxie_ui/src/inspector/handle.rs`'s `Inspect for Handle<T>`,
which compares `AssetDragging.kind` against the inspected field's
`TypeId` and loads a `Handle<T>` onto that field on drop. Dropping a
file over the hierarchy panel today does nothing - nothing there reads
`AssetDragging` at all.

Spawning a `.glb`'s scene into the hierarchy needs a new drop target
that *does* read `AssetDragging`, added to the hierarchy panel (rows
and/or the gap strip in `editor/moxie/src/ui/hierarchy.rs`), mirroring
`Inspect for Handle<T>`'s drop-handling shape but building
`bevy::scene::SceneRoot`/child entities from the gltf's default scene
instead of writing a single field. This is a natural companion to the
folder-browsing item above (same `.glb` registration work), but is
useful on its own even before individual gltf children are browsable -
dropping the whole file can just spawn the default scene.

- [ ] A hierarchy-panel drop target reading `AssetDragging`,
      alongside the existing entity-reparenting one in
      `hierarchy/drag.rs` - same resource, different consumer.
- [ ] On drop, resolve the dragged path to a gltf asset and spawn its
      default scene (or the whole node graph) as children of the drop
      target, parallel to how `Inspect for Handle<T>` resolves a path
      through `AssetServer` with `override_unapproved()` today.
- [ ] Decide undo/naming conventions for a spawned subtree - it's the
      first hierarchy mutation that isn't either hand-built in the
      editor or loaded wholesale from a `.mox`.

## Multiple simultaneous tracks in the timeline

"Multiple tracks" already exists at two different layers, and neither
is a video-editor-style stack a user can freely add to:

- **Runtime** (`crates/motiongfx/src/track.rs`,`timeline.rs`):
  `TrackList`/`Timeline` do hold `Box<[Track]>`, but `curr_index`/
  `target_index` and `set_target_track` describe *switching between*
  tracks (jump to another sequence and play toward/from it), not
  sampling several simultaneously. Repurposing this for layered
  playback would change `queue_actions`'s sampling model, not just add
  UI.
- **Editor UI** (`motiongfx_scene::block::Block`/`Combinator`,
  `editor/moxie/src/block_layout.rs`): `Scene::animation` is one
  `Block` tree; "tracks" only appear as the lanes `block_layout::layout`
  already assigns per sibling under `All`/`Any`/`Flow` - every child of
  one of those combinators gets its own row today
  (`measure_children`), stacked and drawn by `ui/timeline.rs`'s
  `TrackArea`. There's no persistent, user-named, independently
  addable "Track 1 / Track 2" the way a video editor has - what looks
  like stacked lanes is really nested combinator structure, laid out
  automatically.

The second point matters for scoping: a literal free-form track list
(add/remove/reorder independent tracks a user names) means either
changing `Scene`'s schema to hold a flat `Vec<Block>` alongside/instead
of the single `animation` root - touching serialization,
`block_layout.rs`, and every `EditorScene` mutator - or leaning on the
`All`/`Flow` nesting that already lays siblings out as lanes and
building add/remove/reorder affordances directly on top of that
existing behavior. The latter is a much smaller lift since the layout
half is already built; it would mainly need UI to let a user insert/
remove a top-level `All` child and treat it as a named lane, rather
than requiring the tree-editing gestures the timeline doesn't expose
today.

- [ ] Decide the schema question first: reuse `All`/`Flow` top-level
      children as lanes, or give `Scene` an explicit flat track list -
      this determines the size of everything else.
- [ ] If reusing combinator nesting: UI affordances on the timeline
      panel to add/remove/reorder a top-level lane (today's tree has
      no user-facing "insert a sibling block" gesture at all).
- [ ] Lane naming/labeling - `block_layout.rs`'s `combinator_label`
      shows the `Combinator` kind, not a user-chosen name; a track
      stack wants the latter.
- [ ] If simultaneous (not switched) playback is ever needed at
      runtime, not just visually stacked in the editor: revisit
      `Timeline`'s `curr_index`/`target_index` sampling model in
      `crates/motiongfx/src/timeline.rs`, which currently assumes one
      active track at a time.
