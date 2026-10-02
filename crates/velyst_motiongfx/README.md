# Velyst MotionGfx

**Velyst MotionGfx** animates the typeset paths inside a
[Velyst](https://github.com/voxell-tech/velyst) `VelystKanva` with
[Bevy MotionGfx](https://crates.io/crates/bevy_motiongfx).
`KanvaGroup` picks which paths to animate and `KanvaAnim` sets how
they reveal.

`VelystMotionGfxPlugin` runs the animations. It expects `VelystPlugin`
and `BevyMotionGfxPlugin` to be added as well.

```rust
use bevy::prelude::*;
use velyst_motiongfx::prelude::*;

fn spawn_kanva_anim(mut commands: Commands, scene: Entity) {
    // Traces every path inside a `grid-start`..`grid-end` group.
    commands.spawn((
        KanvaGroup::wrap("grid-start", "grid-end").with_target(scene),
        KanvaAnim::trace(0.5),
    ));
}
```

Drive `KanvaAnim::t` from `0.0` to `1.0` with the `Timeline` API to
play the animation.

## Join the community!

You can join us on the [Voxell discord server](https://discord.gg/Mhnyp6VYEQ).

## License

`velyst_motiongfx` is dual-licensed under either:

- MIT License ([LICENSE-MIT](/LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](/LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.
