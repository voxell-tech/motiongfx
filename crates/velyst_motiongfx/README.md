# Velyst MotionGfx

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/voxell-tech/motiongfx#license)
[![Crates.io](https://img.shields.io/crates/v/velyst_motiongfx.svg)](https://crates.io/crates/velyst_motiongfx)
[![Downloads](https://img.shields.io/crates/d/velyst_motiongfx.svg)](https://crates.io/crates/velyst_motiongfx)
[![Docs](https://docs.rs/velyst_motiongfx/badge.svg)](https://docs.rs/velyst_motiongfx/latest/velyst_motiongfx/)
[![CI](https://github.com/voxell-tech/motiongfx/workflows/CI/badge.svg)](https://github.com/voxell-tech/motiongfx/actions)
[![Discord](https://img.shields.io/discord/442334985471655946.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/Mhnyp6VYEQ)

**Velyst MotionGfx** animates [Typst](https://typst.app) content
rendered by [Velyst](https://github.com/voxell-tech/velyst), path by
path, with [Bevy MotionGfx](https://crates.io/crates/bevy_motiongfx).

## Core Concepts

- **`VelystKanva`** holds the paths Velyst draws for a Typst
  function.
- **`KanvaGroup`** picks some of those paths, by labels you write in
  Typst.
- **`KanvaAnim`** reveals the picked paths one after another. Its `t`
  goes from `0.0` (hidden) to `1.0` (shown), and you animate it like
  any other field.

Add `VelystMotionGfxPlugin` next to `VelloPlugin`, `VelystPlugin` and
`BevyMotionGfxPlugin`.

## 1. Label the Paths in Typst

Wrap what you want to animate in two empty labelled boxes:

```typ
#let scene() = canvas(length: 1pt, {
  import draw: *
  content((0, 0), [#box() <grid-start>])
  grid((-200, -200), (200, 200), step: 40)
  content((0, 0), [#box() <grid-end>])
})
```

## 2. Pick Them and Choose an Animation

```rust
# #[path = "docs/scene.rs"] mod _doc; use _doc::*;
fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let scene = commands
        .spawn((
            VelystFunc::new(assets.load("scene.typ"), SceneFunc {}),
            WorldScene::default(),
            VelystKanva::default(),
        ))
        .id();

    commands.spawn((
        KanvaGroup::wrap("grid-start", "grid-end").with_target(scene),
        // Each path takes half of `t`, so they overlap.
        KanvaAnim::trace(0.5),
    ));
}
```

Other presets are `fade`, `trace_fade`, `scale_fade`, `fade_up` and
`scale_pulse`, or build your own from `KanvaPhase`s.

## 3. Play It

Animate `KanvaAnim.t` to `1.0` on a timeline:

```rust
# #[path = "docs/scene.rs"] mod _doc; use _doc::*;
# fn play(
#     mut commands: Commands,
#     mut motiongfx: ResMut<MotionGfxManager>,
#     grid: Entity,
# ) {
let mut b = motiongfx.create_builder();
let track = b
    .act(grid, path!(KanvaAnim.t), |_| 1.0)
    .play(s(2))
    .compile();
let timeline = b.compile(track);

commands.spawn((
    motiongfx.add_timeline(timeline),
    RealtimePlayer::new().with_playing(true),
));
# }
```

See [`velyst_demo.rs`](../../examples/bevy_examples/examples/velyst_demo.rs)
for a full scene, run with
`cargo run -p bevy_examples --example velyst_demo`.

## Join the community!

You can join us on the [Voxell discord server](https://discord.gg/Mhnyp6VYEQ).

## License

`velyst_motiongfx` is dual-licensed under either:

- MIT License ([LICENSE-MIT](../../LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.
