# Peniko MotionGfx

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/voxell-tech/motiongfx#license)
[![Crates.io](https://img.shields.io/crates/v/peniko_motiongfx.svg)](https://crates.io/crates/peniko_motiongfx)
[![Downloads](https://img.shields.io/crates/d/peniko_motiongfx.svg)](https://crates.io/crates/peniko_motiongfx)
[![Docs](https://docs.rs/peniko_motiongfx/badge.svg)](https://docs.rs/peniko_motiongfx/latest/peniko_motiongfx/)
[![Discord](https://img.shields.io/discord/442334985471655946.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/Mhnyp6VYEQ)

[Peniko](https://github.com/linebender/peniko) 2D graphics support for the
[MotionGfx](https://github.com/voxell-tech/motiongfx) framework.

Provides `Interpolation<Peniko>` implementations for common `peniko` and
`kurbo` types, and a `Trace` trait for draw-on path animations via
`Tracer<T>`.

## Interpolation

Animate `peniko` types like `Point`, `Circle`, `Color`, and curves:

```rust
use peniko_motiongfx::prelude::*;

let a = kurbo::Point::new(0.0, 0.0);
let b = kurbo::Point::new(100.0, 100.0);
let mid = kurbo::Point::interp(&a, &b, 0.5);

assert_eq!(mid, kurbo::Point::new(50.0, 50.0));
```

## Tracing

Draw paths on screen by animating a visible range:

```rust
use peniko_motiongfx::prelude::*;

let mut tracer = LineTracer {
    path: kurbo::Line::new((0., 0.), (100., 0.)),
    t_start: 0.0,
    t_end: 0.0,
};

tracer.t_end = 0.5;
let half_line = tracer.trace();

assert_eq!(half_line, kurbo::Line::new((0., 0.), (50., 0.)));
```

## License

`peniko_motiongfx` is dual-licensed under either:

- MIT License ([LICENSE-MIT](/LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](/LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.
