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
Trace a graph's grid line by line, fade an equation in, or pop a
shape into place.

## How It Works

Velyst lays out a Typst function and draws it into a `VelystKanva`, a
flat list of paths. This crate adds two components that sit beside
it:

- `KanvaGroup` picks which paths to animate, using labels you write
  in Typst.
- `KanvaAnim` says how those paths appear. Its `t` goes from `0.0`
  (hidden) to `1.0` (fully shown), and you animate `t` with a
  MotionGfx timeline like any other field.

The steps below build a small scene: a grid that traces in, followed
by a circle that scales in.

## 1. Add the Plugins

`VelystMotionGfxPlugin` needs Vello, Velyst and Bevy MotionGfx
alongside it.

```rust,no_run
use bevy::prelude::*;
use bevy_motiongfx::BevyMotionGfxPlugin;
use velyst_motiongfx::prelude::*;
use velyst_motiongfx::velyst::VelystPlugin;
use velyst_motiongfx::velyst::bevy_vello::VelloPlugin;

App::new()
    .add_plugins((
        DefaultPlugins,
        VelloPlugin::default(),
        BevyMotionGfxPlugin,
        VelystPlugin,
        VelystMotionGfxPlugin,
    ))
    .run();
```

## 2. Label What You Want to Animate

In your Typst file, mark each part you want to animate on its own.
Put an empty labelled box before and after it:

```typ
// assets/scene.typ
#import "@preview/cetz:0.5.2": canvas, draw

#let scene() = canvas(length: 1pt, {
  import draw: *

  content((0, 0), [#box() <grid-start>])
  grid((-200, -200), (200, 200), step: 40, stroke: gray)
  content((0, 0), [#box() <grid-end>])

  content((0, 0), [#box() <circle-start>])
  circle((0, 0), radius: 20, fill: purple)
  content((0, 0), [#box() <circle-end>])
})
```

Every path drawn between `<grid-start>` and `<grid-end>` now belongs
to the grid. You can also put a label on the content itself, such as
`#box[$x^2$] <eq>`, to select everything inside it.

## 3. Spawn the Typst Function

Declare the Typst function with `typst_func!`, register it, and spawn
it with a `VelystKanva` so its paths can be animated. A camera with
`VelloView` renders it.

```rust,no_run
# use bevy::prelude::*;
# use velyst_motiongfx::prelude::*;
use velyst_motiongfx::velyst::bevy_vello::prelude::*;

// Matches `#let scene() = ...` in `scene.typ`.
typst_func!("scene", #[derive(Default, Clone)] struct SceneFunc {});

# fn plugin(app: &mut App) {
// When building the app:
app.register_typst_func::<SceneFunc>();
# }

fn spawn_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((Camera2d, VelloView));

    commands.spawn((
        VelystFunc::new(
            asset_server.load("scene.typ"),
            SceneFunc::default(),
        ),
        WorldScene::default().with_anchor(Vec2::splat(0.5)),
        VelystKanva::default(),
    ));
}
```

## 4. Pick the Paths

Spawn an entity with a `KanvaGroup` for each part, pointing at the
scene with `with_target`.

| Constructor | Selects |
| --- | --- |
| `KanvaGroup::all()` | every path in the kanva |
| `KanvaGroup::inner("eq")` | the paths inside the content labelled `<eq>` |
| `KanvaGroup::wrap("grid-start", "grid-end")` | the paths between two marker labels |

Without `with_target`, the group animates the kanva on its own
entity, so a single-part scene can put everything on one entity.

## 5. Choose How They Appear

Add a `KanvaAnim` next to each `KanvaGroup`. Each preset describes
what one path does as it appears:

| Preset | Each path |
| --- | --- |
| `KanvaAnim::trace(w)` | draws its outline from start to end |
| `KanvaAnim::fade(w)` | fades in |
| `KanvaAnim::trace_fade(w, r)` | traces, then fades its fill in (`r` is the share spent tracing) |
| `KanvaAnim::scale_fade(w)` | grows from half size while fading in |
| `KanvaAnim::fade_up(w)` | slides up while fading in |
| `KanvaAnim::scale_pulse(w)` | pops bigger, then settles back |

Paths appear one after another rather than all at once. The `w`
argument (`path_window`) sets how much of the animation each path
takes, from `0.0` to `1.0`:

- `1.0`: every path animates together.
- `0.5`: each path takes half the time, so the paths overlap.
- Close to `0.0`: each path snaps in, one after another.

```rust
# use bevy::prelude::*;
# use velyst_motiongfx::prelude::*;
fn spawn_parts(mut commands: Commands, scene: Entity) {
    commands.spawn((
        KanvaGroup::wrap("grid-start", "grid-end")
            .with_target(scene),
        KanvaAnim::trace(0.5),
    ));
    commands.spawn((
        KanvaGroup::wrap("circle-start", "circle-end")
            .with_target(scene),
        KanvaAnim::scale_fade(0.3),
    ));
}
```

## 6. Play It

Animate each part's `KanvaAnim.t` to `1.0` on a timeline. `ord_chain`
plays them one after another.

```rust
# use bevy::prelude::*;
use bevy_motiongfx::prelude::*;
# use velyst_motiongfx::prelude::*;

fn play(
    mut commands: Commands,
    mut motiongfx: ResMut<MotionGfxManager>,
    grid: Entity,
    circle: Entity,
) {
    let mut b = motiongfx.create_builder();

    let track = [
        b.act(grid, path!(KanvaAnim.t), |_| 1.0)
            .with_ease(ease::cubic::ease_in_out)
            .play(s(2)),
        b.act(circle, path!(KanvaAnim.t), |_| 1.0)
            .with_ease(ease::cubic::ease_in_out)
            .play(s(1)),
    ]
    .ord_chain()
    .compile();

    let timeline = b.compile(track);
    commands.spawn((
        motiongfx.add_timeline(timeline),
        RealtimePlayer::new().with_playing(true),
    ));
}
```

## Animating Typst Arguments

The fields of a `typst_func!` struct are the Typst function's
arguments, and they can be animated too. Velyst lays the content out
again whenever they change.

```typ
#let plot(circle_x, circle_y) = canvas(length: 1pt, {
  import draw: *
  circle((circle_x * 40, circle_y * 40), radius: 20)
})
```

```rust
# use bevy::prelude::*;
use bevy_motiongfx::prelude::*;
# use velyst_motiongfx::prelude::*;

typst_func!(
    "plot",
    #[derive(Default, Clone)]
    struct PlotFunc {},
    positional_args { circle_x: f64, circle_y: f64 }
);

type VPlotFunc = VelystFunc<PlotFunc>;

# fn act(b: &mut BevyTimelineBuilder, plot: Entity) {
// Moves the circle 3 steps along the grid.
b.act(plot, path!(VPlotFunc.data.circle_x), |_| 3.0)
    .play(s(2));
# }
```

## Custom Animations

Each preset is a list of `KanvaPhase`s. A phase is a function that
changes one path for a given `t`, plus the part of that path's
animation it runs in. Build your own `KanvaAnim` from them:

```rust
# use velyst_motiongfx::prelude::*;
use velyst_motiongfx::{alpha_phase, trace_phase};

// Trace for the first 70%, then fade the whole path in.
let anim = KanvaAnim {
    t: 0.0,
    path_window: 0.4,
    phases: vec![
        KanvaPhase::new(trace_phase, 0.0, 0.7),
        KanvaPhase::new(alpha_phase, 0.7, 1.0),
    ],
};
```

## Full Example

[`velyst_demo.rs`](../../examples/bevy_examples/examples/velyst_demo.rs)
puts all of this together with a coordinate plot from
[`velyst_demo.typ`](../../examples/bevy_examples/assets/typst/velyst_demo.typ):

```sh
cargo run -p bevy_examples --example velyst_demo
```

## Join the community!

You can join us on the [Voxell discord server](https://discord.gg/Mhnyp6VYEQ).

## License

`velyst_motiongfx` is dual-licensed under either:

- MIT License ([LICENSE-MIT](../../LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.
