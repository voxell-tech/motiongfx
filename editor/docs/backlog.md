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
