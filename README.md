# MotionGfx

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/voxell-tech/motiongfx#license)
[![Crates.io](https://img.shields.io/crates/v/motiongfx.svg)](https://crates.io/crates/motiongfx)
[![Downloads](https://img.shields.io/crates/d/motiongfx.svg)](https://crates.io/crates/motiongfx)
[![Docs](https://docs.rs/motiongfx/badge.svg)](https://docs.rs/motiongfx/latest/motiongfx/)
[![CI](https://github.com/voxell-tech/motiongfx/workflows/CI/badge.svg)](https://github.com/voxell-tech/motiongfx/actions)
[![Discord](https://img.shields.io/discord/442334985471655946.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/Mhnyp6VYEQ)

**MotionGfx** is a backend-agnostic motion graphics creation framework
for Rust. Free and open-source forever.

[Documentation](https://motiongfx.voxell.dev) ·
[API reference](https://docs.rs/motiongfx) ·
[Discord](https://discord.gg/Mhnyp6VYEQ)

## Relative, not absolute

An action's closure receives the field's current value, so each step
picks up wherever the last one left off.

```rust
let deltas = [
    Vec2 { x: 180.0, y: 50.0 },
    Vec2 { x: 180.0, y: -100.0 },
    Vec2 { x: 180.0, y: 50.0 },
];
let tracks = deltas.map(|delta| {
    b.act(dot, path!(<Dot>::position), move |p| *p + delta)
        .with_ease(ease::cubic::ease_in_out)
        .play(cs(50))
});

let track = tracks.ord_chain().compile();
```

## It's just code

A scene is plain Rust, built with the same loops and variables you
already use. Here, ten bars share one `.map()` and one stagger.

```rust
let stagger = ms(60);
let tracks = HEIGHTS
    .iter()
    .enumerate()
    .map(|(i, &height)| {
        [
            b.act(i, path!(<Bar>::height), move |_| height)
                .with_ease(ease::cubic::ease_in_out)
                .play(ms(600)),
            b.act(i, path!(<Bar>::y), move |_| {
                BASELINE - height / 2.0
            })
            .with_ease(ease::cubic::ease_in_out)
            .play(ms(600)),
        ]
        .ord_all()
    })
    .collect::<Vec<_>>();

let track = tracks.ord_flow(stagger).compile();
```

## Scrub forward and backward, for free

A timeline bakes once. After that it plays at any speed, in either
direction, or jumps straight to any frame without re-simulating
anything.

```rust
let track = [
    b.act(ball, path!(<Ball>::y), |_| 30.0)
        .with_ease(ease::quad::ease_out)
        .play(ms(600)),
    b.act(ball, path!(<Ball>::y), |_| 140.0)
        .with_ease(ease::quad::ease_in)
        .play(ms(600)),
]
.ord_chain()
.compile();
```

Try these live in the
[playground](https://motiongfx.voxell.dev).

## Backend agnostic

MotionGfx describes what changes and leaves drawing to the backend.
Bevy is supported today, and any renderer that can read and write its
own values can be next.

```text
                MotionGfx
                    │
      ┌─────────────┼─────────────┐
      ▼             ▼             ▼
    Bevy      Your renderer      ...
```

Building your own? See the
[backend guide](https://motiongfx.voxell.dev/docs/advanced).

## Start with Bevy

[Bevy MotionGfx](https://crates.io/crates/bevy_motiongfx) handles the
setup for you. Add the plugin, describe what should change, and it
plays. See the
[setup guide](https://motiongfx.voxell.dev/docs/bevy).

## Prefer a timeline you can see?

[Moxie](https://github.com/voxell-tech/moxie) is a Bevy editor for
MotionGfx, with a hierarchy, an inspector, and a scrubbable timeline
built on the same chain, all, and flow combinators. You arrange them
visually, without writing code.

## Learn more

The [crate docs](https://docs.rs/motiongfx) walk through the world,
registry, timeline builder, and track ordering step by step.

## Community

Join the [Voxell discord server](https://discord.gg/Mhnyp6VYEQ).

## Inspirations and similar projects

- [Motion Canvas](https://motioncanvas.io/)
- [Manim](https://www.manim.community/)

## License

`motiongfx` is dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or
  [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer! This dual-licensing
approach is the de-facto standard in the Rust ecosystem and there are
[very good reasons](https://github.com/bevyengine/bevy/issues/2373) to
include both.
