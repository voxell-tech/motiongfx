# Peniko MotionGfx

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/voxell-tech/motiongfx#license)
[![Crates.io](https://img.shields.io/crates/v/peniko_motiongfx.svg)](https://crates.io/crates/peniko_motiongfx)
[![Downloads](https://img.shields.io/crates/d/peniko_motiongfx.svg)](https://crates.io/crates/peniko_motiongfx)
[![Docs](https://docs.rs/peniko_motiongfx/badge.svg)](https://docs.rs/peniko_motiongfx/latest/peniko_motiongfx/)
[![CI](https://github.com/voxell-tech/motiongfx/workflows/CI/badge.svg)](https://github.com/voxell-tech/motiongfx/actions)
[![Discord](https://img.shields.io/discord/442334985471655946.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/Mhnyp6VYEQ)

**Peniko MotionGfx** adds
[MotionGfx](https://crates.io/crates/motiongfx) support for the 2D
types from [`peniko`](https://crates.io/crates/peniko) and
[`kurbo`](https://crates.io/crates/kurbo).

## Interpolation

`Interpolation<Peniko>` is implemented for `Color` and the `kurbo`
shapes - `Point`, `Vec2`, `Size`, `Rect`, `RoundedRect`, `Circle`,
`Line`, `QuadBez`, `CubicBez` - so they animate like any other value.
`Peniko` is the local marker the orphan rule needs.

## Tracing

`trace` slices a curve or `BezPath` to a visible sub-range: `Tracer`
holds the full path plus a normalized `t_start..t_end`, and `trace()`
returns just that segment - a draw-on effect when the range is
animated. `LineTracer`, `QuadTracer`, `CubicTracer`, and `PathTracer`
are the ready-made aliases.

`#![no_std]`.

## Join the community!

You can join us on the [Voxell discord server](https://discord.gg/Mhnyp6VYEQ).

## License

`peniko_motiongfx` is dual-licensed under either:

- MIT License ([LICENSE-MIT](/LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](/LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.
