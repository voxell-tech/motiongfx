# Bevy MotionGfx

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/voxell-tech/motiongfx#license)
[![Crates.io](https://img.shields.io/crates/v/bevy_motiongfx.svg)](https://crates.io/crates/bevy_motiongfx)
[![Downloads](https://img.shields.io/crates/d/bevy_motiongfx.svg)](https://crates.io/crates/bevy_motiongfx)
[![Docs](https://docs.rs/bevy_motiongfx/badge.svg)](https://docs.rs/bevy_motiongfx/latest/bevy_motiongfx/)
[![Discord](https://img.shields.io/discord/442334985471655946.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/Mhnyp6VYEQ)

An integration of the
[MotionGfx](https://github.com/voxell-tech/motiongfx) crate for the
[Bevy](https://bevyengine.org) game engine.

## Usage

### Initialization

The `BevyMotionGfxPlugin` must be added for timeline and controllers
to work.

```rust,no_run
use bevy::prelude::*;
use bevy_motiongfx::BevyMotionGfxPlugin;

App::new()
    .add_plugins((DefaultPlugins, BevyMotionGfxPlugin))
    // Add systems here...
    .run();
```

### Create Animations

For a more thorough walkthrough on the `Timeline` API, read the
[MotionGfx docs](https://docs.rs/motiongfx/latest/motiongfx).

This example demonstrates how to animate an `Entity`.

```rust
use bevy::prelude::*;
use bevy_motiongfx::prelude::*;

fn build_timeline(
    mut commands: Commands,
    mut motiongfx: ResMut<MotionGfxWorld>,
) {
    // Spawn the Entity.
    let entity = commands
        .spawn(Transform::from_xyz(-3.0, 0.0, 0.0))
        .id();

    // Build the timeline.
    let mut b = TimelineBuilder::new();
    let track = b
        .act_interp(entity, field!(<Transform>::translation::x), |x| {
            x + 6.0
        })
        .play(1.0)
        .compile();

    b.add_tracks(track);
    let timeline = b.compile();

    // Spawn the timeline.
    commands.spawn(motiongfx.add_timeline(timeline));
}
```

This example demonstrates how to animate an `Asset`.

```rust
use bevy::prelude::*;
use bevy_motiongfx::prelude::*;

fn build_timeline(
    mut commands: Commands,
    mut motiongfx: ResMut<MotionGfxWorld>,
    mut materials: ResMut<Assets<StandardMaterial>>
) {
    // Create the asset.
    let material =
        materials.add(StandardMaterial::from_color(Srgba::BLUE));
    // Spawn an entity to prevent the asset from dropping.
    commands.spawn(MeshMaterial3d(material.clone()));

    // Build the timeline.
    let mut b = TimelineBuilder::new();
    let track = b
        .act_interp(
          // AssetIds must be type-erased.
          material.untyped().id(),
          field!(<StandardMaterial>::base_color),
          |_| Srgba::RED.into(),
        )
        .play(1.0)
        .compile();

    b.add_tracks(track);
    let timeline = b.compile();

    // Spawn the timeline.
    commands.spawn(motiongfx.add_timeline(timeline));
}
```

### Controllers

Controllers are helper components for automating the target time and
target track of a `Timeline`.

```rust
use bevy::prelude::*;
use bevy_motiongfx::prelude::*;

fn build_timeline(
    mut commands: Commands,
    mut motiongfx: ResMut<MotionGfxWorld>,
) {
    // Build the timeline.
    let mut b = TimelineBuilder::new();
    // Add tracks here...
    let timeline = b.compile();

    // Spawn the timeline with a controller.
    commands.spawn((
        motiongfx.add_timeline(timeline),
        RealtimePlayer::new().with_playing(true),
    ));
}
```

## Version Matrix

| Bevy    | MotionGfx  | Bevy MotionGfx  |
| ------- | ---------- | --------------- |
| 0.18    | 0.2        | 0.2             |
| 0.17    | 0.1        | 0.1             |

## License

`bevy_motiongfx` is dual-licensed under either:

- MIT License ([LICENSE-MIT](/LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](/LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.

