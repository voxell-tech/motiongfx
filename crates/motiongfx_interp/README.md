# MotionGfx Interp

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/voxell-tech/motiongfx#license)
[![Crates.io](https://img.shields.io/crates/v/motiongfx_interp.svg)](https://crates.io/crates/motiongfx_interp)
[![Downloads](https://img.shields.io/crates/d/motiongfx_interp.svg)](https://crates.io/crates/motiongfx_interp)
[![Docs](https://docs.rs/motiongfx_interp/badge.svg)](https://docs.rs/motiongfx_interp/latest/motiongfx_interp/)
[![CI](https://github.com/voxell-tech/motiongfx/workflows/CI/badge.svg)](https://github.com/voxell-tech/motiongfx/actions)
[![Discord](https://img.shields.io/discord/442334985471655946.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/Mhnyp6VYEQ)

**MotionGfx Interp** is the math layer behind
[MotionGfx](https://crates.io/crates/motiongfx): easing curves and
value interpolation, with nothing else attached. It lives in its own
crate so projects that only need to move a number from `a` to `b` can
depend on it without pulling in the animation runtime.

The crate is two independent pieces.

## Easing

`ease` holds pure shaping functions of type `fn(f32) -> f32`. Each
takes a progress value in `0.0..=1.0` and returns a reshaped one in
the same range, anchored at `0.0` and `1.0`. `ease::linear` is the
identity; every other curve is grouped into a module by family, each
exposing `ease_in`, `ease_out`, and `ease_in_out`.

```rust
use motiongfx_interp::ease;

let p = ease::cubic::ease_in_out(0.25);
```

Families: `sine`, `quad`, `cubic`, `quart`, `quint`, `expo`, `circ`,
`back`, `elastic`. They follow the definitions from
[easings.net](https://easings.net/).

## Interpolation

`Interpolation` blends two values of one type by a progress value —
typically the output of an easing function.

```rust
use motiongfx_interp::interpolation::Interpolation;

let x = f32::interp(&0.0, &10.0, 0.25); // 2.5
```

It is implemented for `f32`, `f64`, `i32`, `u32`, and `u8`. Integers
are computed through `f64` and rounded, so the endpoints stay exact at
`t == 0.0` and `t == 1.0`.

The trait carries a marker type parameter:

```rust,ignore
pub trait Interpolation<M> { /* ... */ }
```

`M` exists only for the orphan rule. A downstream crate that wants to
interpolate a type it does not own can implement the trait against a
local marker instead. The `impl_float_interpolation!` and
`impl_int_interpolation!` macros generate these impls, with or without
a marker.

## `no_std`

The `std` feature is on by default and uses the standard library's
float intrinsics. Disable it and the same math routes through
[`libm`](https://crates.io/crates/libm), leaving the crate `no_std`.

## Join the community!

You can join us on the [Voxell discord server](https://discord.gg/Mhnyp6VYEQ).

## License

`motiongfx_interp` is dual-licensed under either:

- MIT License ([LICENSE-MIT](/LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](/LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.
