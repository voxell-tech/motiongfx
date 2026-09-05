# Headless UI checks

How the number-field bugs (stray text input over the top bar, stacked
value text) were reproduced and verified without clicking anything.
This is a manual harness, not a test suite. Notes for building the
real one are at the end.

## The loop

1. Add a throwaway diagnostic system to `UiPlugin`
   (`editor/moxie/src/ui.rs`).
2. `cargo build -p moxie`, run the binary with its output to a file,
   `sleep`, `pkill`.
3. Read the log.
4. To compare against the unfixed code: `git stash push -m <tag>` the
   fix, rebuild, rerun, diff the logs, `git stash pop`.
5. `git checkout` every diagnostic edit. Only the fix stays.

The binary still opens a real window (`DefaultPlugins`), so this needs
a display. It needs no interaction.

## Driving state

The panels are reactive: each rebuilds when a resource it watches
changes. So a diagnostic system sets those resources directly rather
than faking pointer input.

```rust
fn __diag(
    mut phase: Local<u8>,
    time: Res<Time>,
    mut commands: Commands,
    mut sel: ResMut<SelectedAction>,
) {
    match (*phase, time.elapsed_secs()) {
        (0, t) if t > 3.0 => {
            commands.queue(crate::project::__diag_load);
            *phase = 1;
        }
        (1, t) if t > 6.0 => {
            sel.0 = Some(vec![0, 0]);
            *phase = 2;
        }
        (2, t) if t > 9.0 => { /* dump */ *phase = 3; }
        _ => {}
    }
}
```

Phases run off wall-clock seconds because the kernel needs several
frames to flush a rebuild and settle its bindings and tweens.

### Loading a project without the dialog

`project::load_scene` calls `ask_for_path`, which opens `rfd` and
blocks. A diagnostic copy skips it:

```rust
pub(crate) fn __diag_load(world: &mut World) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/projects/hello_world/hello_world.mox");
    let text = std::fs::read_to_string(&path).unwrap();
    let Some(project) = deserialize(world, &text, &path) else { return };
    clear(world);
    let registry = world.resource::<AppTypeRegistry>().clone();
    project.world
        .write_to_world_with(world, &mut default(), &registry.read())
        .unwrap();
    world.insert_resource(EditorScene::new(MotionGfxScene(project.scene)));
}
```

Run the binary from `editor/moxie` so the `../assets` source resolves.

## Reading state

Query the world instead of looking at pixels.

Orphaned nodes (the stray input): a `ChildOf`-less UI node lays out
from the window origin.

```rust
Query<(Entity, &bevy::ui::UiGlobalTransform, Option<&ChildOf>),
      With<bevy::text::EditableText>>
// orphan == child_of.is_none(); dump translation + ComputedNode::size
```

Event storms: a global observer that logs each fire.

```rust
app.add_observer(|u: On<UpdateNumberInput>| {
    info!("UpdateNumberInput -> {} = {:?}", u.entity, u.value);
});
```

Widget content: `EditableText::value().to_string()`.

## Running

```sh
cargo build -p moxie
cd editor/moxie
(RUST_LOG=warn,moxie=info ../../target/debug/moxie > /tmp/run.log 2>&1 &)
sleep 24
pkill -f target/debug/moxie
grep -E "orphan|EditableText value|UpdateNumberInput" /tmp/run.log
```

`RUST_LOG=warn` drops the wgpu/winit noise; `moxie=info` keeps the
diagnostic lines.

## What a real harness needs

- Headless run: `DefaultPlugins` opens a window. A test build wants
  `bevy_ci_testing` / `ScheduleRunnerPlugin`, or a headless render
  backend.
- A test-only entry point for loading a project by path, so the
  diagnostic copy of `load_scene` is not needed.
- Frame stepping: call `app.update()` in a loop until a condition
  holds, not `sleep` against wall-clock phases.
- Assertions over log grep.
- Enough frames after each state change for the kernel to flush the
  rebuild and its bindings.
