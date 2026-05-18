# velyst_motiongfx

Velyst animation support for the MotionGfx framework.

This crate provides components and systems for animating Velyst typesetting graphics using the MotionGfx timeline-based animation framework.

## Quick Start

Add the plugin to your Bevy app:

```rust
use velyst_motiongfx::prelude::*;

app.add_plugins(VelystMotionGfxPlugin);
```

Spawn a traced animation on a Velyst scene entity:

```rust
let scene = commands.spawn((
    VelystFunc::new(handle, MyTypstFunc::default()),
    VelystKanva::default(),
)).id();

let trace = commands.spawn(TraceKanva {
    kanva: Some(scene),
    group: KanvaGroup::Wrap("grid-start", "grid-end"),
    ..default()
}).id();

// Drive the animation with MotionGfx timeline
b.act(trace, path!(<TraceKanva>::t), |_| 1.0)
    .with_ease(ease::cubic::ease_in_out)
    .play(3.0);
```

For paths that fade in while tracing:

```rust
commands.spawn(TraceFadeKanva {
    kanva: Some(scene),
    trace_ratio: 0.6,  // 60% tracing, 40% fading
    ..default()
});
```

## Components

- `TraceKanva` -- Draws paths with a moving trace point. Fields:
  - `t`: Current trace progress (0..1)
  - `path_window`: Visible trace window width (0..1)
  - `kanva`: Target entity with `VelystKanva` (uses self if None)
  - `group`: Which paths to animate

- `TraceFadeKanva` -- Combines tracing with alpha fade-in. Fields:
  - `t`: Current animation progress
  - `path_window`: Animation window duration
  - `trace_ratio`: Fraction of window for trace phase (rest is fade)
  - `kanva`: Target entity with `VelystKanva`
  - `group`: Which paths to animate

## Path Selection

Use `KanvaGroup` to choose which paths to animate:

- `All` -- Every path in the kanva
- `Inner("name")` -- Paths inside a labeled group
- `Wrap("start", "end")` -- Paths between two marker groups
