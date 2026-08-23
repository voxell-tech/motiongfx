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

## `.watch()`'s first build waits a flush it doesn't need to

`ElementMut::watch`/`Fynix::watch` only register a watcher; the first
build happens later, whenever `flush()` first polls it. Every
`fynix_mock` predicate (`component_changed_on`, `resource_changed`,
`value_changed`, `shape_changed`) is written to always report changed
on its first call, which is what stands in for an immediate build -
but `flush()` only merges a watcher registered mid-build
(`Records::spawned`) into the live list after that flush's loop
finishes, so a `.watch()` reached from inside another one's build
waits a whole flush before it gets its own first poll. Nested a few
levels deep - a hierarchy row's body watching its own children, whose
body watches its own - a full rebuild of an already-open subtree (a
sibling reordered, a sibling added) visibly settles one level, one
frame, at a time instead of appearing whole.

Real fynix (`~/develop/projects/rust/fynix`) doesn't have this: its
`ctx.watch()` (`crates/fynix/src/ctx.rs`) builds the subtree inline,
synchronously, right there, and only arms the watcher for later
changes afterward.

- [x] `ElementMut::watch` and `Fynix::watch` now poll `changed` once
      immediately and build right there if it fires, rather than only
      registering for `flush()` to find later. Simpler than the fix
      first sketched here: that one built unconditionally and called
      `changed` a second time only to consume its guaranteed-true
      first call; polling first and gating the build on the result
      needs one call, not two, and also does the right thing for a
      predicate that legitimately starts out false (nothing builds
      until it actually fires) - `Fynix::watch` gained a `world: &mut
      H::World` parameter for this, so `bevy_fynix::watch_root` now
      reaches it through `World::resource_scope` rather than
      `resource_mut`, the same way `with_kernel` already did.
- [x] Both call `clear_children` before that immediate build, not
      only `flush()`'s loop - missing from the first sketch, and
      necessary: a node can already have children when `.watch()` is
      called (`unwatch` then `watch` again, say), and skipping it
      would build a second subtree alongside the first instead of
      replacing it.
- [x] `crates/fynix_mock/tests/kernel.rs`, `transitions.rs`,
      `styles.rs` updated for the new signature and behavior; the
      whole suite passes.
- [ ] Verify in the running editor: expanding several nested hierarchy
      rows and asset folders, then reordering or adding a sibling near
      the root - the whole open subtree should reappear in one frame,
      not visibly settle level by level.

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
- [x] A `Handle<T>` branch in the reflect-tree inspector: done as
      `moxie_ui::inspector::handle`, `impl<T: Asset + TypePath>
      Inspect for Handle<T>`, so it slots into the existing walk
      rather than needing its own dispatch. Registered so far only for
      `Handle<StandardMaterial>`; still needs a browse button beside
      the drop target, reusing the `rfd::` dialog already in
      `project.rs`.
- [x] Storage that outlives the load call: `source.write` puts the
      freshly loaded handle straight into the component field, so the
      component itself is what holds it, not a local that would drop
      it.
- [ ] A preview panel beside the asset browser's own listing, showing
      whatever asset is currently selected or hovered there.
