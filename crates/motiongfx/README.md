# MotionGfx

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/voxell-tech/motiongfx#license)
[![Crates.io](https://img.shields.io/crates/v/motiongfx.svg)](https://crates.io/crates/motiongfx)
[![Downloads](https://img.shields.io/crates/d/motiongfx.svg)](https://crates.io/crates/motiongfx)
[![Docs](https://docs.rs/motiongfx/badge.svg)](https://docs.rs/motiongfx/latest/motiongfx/)
[![CI](https://github.com/voxell-tech/motiongfx/workflows/CI/badge.svg)](https://github.com/voxell-tech/motiongfx/actions)
[![Discord](https://img.shields.io/discord/442334985471655946.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/Mhnyp6VYEQ)

**MotionGfx** is a backend-agnostic motion graphics framework
built on top of [Bevy](https://bevyengine.org) ECS. It provides a
modular foundation for procedural animations.

## Key Features

- **Backend agnostic**: Works with any rendering backend.
- **Procedural**: Write animations with code, loops, functions,
  logic.
- **Type-erased**: Powered by
  [Field Path](https://github.com/voxell-tech/field_path), allowing
  runtime-flexible animation of arbitrary data.
- **Two-way playback**: Play animations both forward and backward with
  no extra computation.
- **Batteries included**: Packed with common easing and interpolation
  functions.

## Quick Start

```rust
use motiongfx::prelude::*;

// A marker is a plain unit struct, a type-level tag that identifies
// this particular SubjectSource implementation. It holds no data.
// You need it because you can't implement foreign traits (SubjectSource)
// on foreign types (Vec<f32>) without a disambiguating marker.
struct Marker;

// The actual data lives in Vec<f32>. We teach MotionGfx how to read
// and write f32 subjects from it, tagged by `Marker`.
impl SubjectSource<Marker, usize, f32> for Vec<f32> {
    fn get_source(&self, id: usize) -> Option<&f32> {
        self.get(id)
    }

    fn apply_source<R>(
        &mut self,
        id: usize,
        f: impl FnOnce(&mut f32) -> R,
    ) -> Option<R> {
        self.get_mut(id).map(f)
    }
}

let mut subjects = vec![0.0_f32];

// The registry tracks which types are animated.
let mut registry = Registry::new();
// The builder is typed on the data holder (Vec<f32>).
let mut b = registry.create_builder::<Vec<f32>>();

let id = 0;
// Create an action with: id, field path, action fn.
let action = b
    // Animate subject 0 from its current value to +10.0.
    .act(id, path!(<f32>), |x| x + 10.0)
    // Every action needs an interpolation function.
    .with_interp(|a, b, t| a + (b - a) * t);

// "Play" the action into a `TrackFragment` with a duration.
let frag = action.play(1.0);

// Compile into a `Track` (see Track Ordering for composing fragments).
let track = frag.compile();

b.add_tracks(track);
let mut timeline = b.compile();

// Bake must run once before sampling.
timeline.bake_actions(&registry, &subjects);

// Sample at t = 0.5, subjects[0] should now be 5.0.
timeline.set_target_time(0.5);
timeline.queue_actions();
timeline.sample_queued_actions(&registry, &mut subjects);

assert!((subjects[0] - 5.0).abs() < f32::EPSILON);
```

## Creating your first animation

### The World

MotionGfx separates the **marker** from the **data holder**:

- The **marker** (`M`) is a plain unit struct with no fields. It is a
  type-level tag that identifies a specific `SubjectSource`
  implementation. This lets you have multiple independent impls on the
  same data type without conflicting, and works around the orphan rule
  when the data type is not yours.
- The **data holder** (`W`) is whatever owns the subjects, a `Vec`,
  a `HashMap`, your own struct. This is the type you pass to
  `bake_actions` and `sample_queued_actions`.

```rust
# use motiongfx::prelude::*;
// The marker: no data, just a type-level tag.
struct Marker;

// The data holder: subjects are f32 values, keyed by index.
impl SubjectSource<Marker, usize, f32> for Vec<f32> {
    fn get_source(&self, id: usize) -> Option<&f32> {
        self.get(id)
    }

    fn apply_source<R>(
        &mut self,
        id: usize,
        f: impl FnOnce(&mut f32) -> R,
    ) -> Option<R> {
        self.get_mut(id).map(f)
    }
}
```

### The Registry

The `Registry` keeps track of how to animate your types behind the
scenes. MotionGfx is type-erased at runtime, so it needs the registry
to know how to read and write each field. You don't need to set it up
manually, just create one and pass it to the builder. Registration
happens automatically the first time you add an animation.

```rust
use motiongfx::prelude::*;

let mut registry = Registry::new();
```

### The Timeline Builder

The `TimelineBuilder` is where you describe your animations. Create
one from the registry, typed to your data holder:

```rust
# #[path = "docs/world.rs"] mod _doc;
# use _doc::*;
# let mut registry = registry();
let mut b = registry.create_builder::<Vec<f32>>();
```

### Building the Timeline

Animations are built up in layers:

1. An **action** says what to animate and how to transform it.
2. A **track fragment** gives the action a duration by calling `.play(seconds)`.
3. A **track** is one or more fragments compiled together. You can
   order fragments before compiling (see [Track Ordering](#track-ordering)).
4. A **timeline** combines all your tracks into one playable sequence.

```rust
# #[path = "docs/world.rs"] mod _doc;
# use _doc::*;
# let mut registry = registry();
# let mut b = registry.create_builder::<Vec<f32>>();
let id = 0;
// Act: animate subject 0 from its current value to +10.0.
let action = b
    .act(id, path!(<f32>), |x| x + 10.0)
    // Every action needs an interpolation function.
    .with_interp(|a: &f32, b: &f32, t| a + (b - a) * t)
    // An optional easing function can be added.
    .with_ease(ease::cubic::ease_in_out);

// Play: turn the action into a fragment with a 1-second duration.
let frag = action.play(1.0);

// Compile the fragment into a Track.
let track = frag.compile();

// Add the track and compile into a Timeline.
b.add_tracks(track);
let mut timeline = b.compile();
```

### Bake and Sample

Before playing an animation, you need to **bake** it. Baking reads
the starting values from your world and prepares the animation data.
This only needs to happen once, right after building the timeline.

To advance the animation, set a target time, then **queue** and
**sample**. Queuing figures out which actions are active at that
time, and sampling writes the new values back into your world.

A timeline can also have multiple tracks, each acting as a chapter.
Use `set_target_track` to jump between them.

```rust
# #[path = "docs/world.rs"] mod _doc;
# use _doc::*;
# let mut subjects = vec![0.0_f32];
# let (registry, mut timeline) = timeline();
// Bake once after building the timeline.
timeline.bake_actions(&registry, &subjects);

// Set target time, queue, then sample.
timeline.set_target_time(0.5);
timeline.queue_actions();
timeline.sample_queued_actions(&registry, &mut subjects);

assert!((subjects[0] - 5.0).abs() < f32::EPSILON);
```

### Track Ordering

You can control how fragments play relative to each other. There are
4 ordering combinators:

#### 1. Chain

```rust
use motiongfx::prelude::*;

// Using empty fragments as an example only.
let f0 = TrackFragment::new();
let f1 = TrackFragment::new();

let f = [f0, f1].ord_chain();
// Or...
// use motiongfx::track::chain;
// let f = chain([f0, f1]);
```

`f1` plays after `f0` finishes.

#### 2. All

```rust
use motiongfx::prelude::*;

let f0 = TrackFragment::new();
let f1 = TrackFragment::new();

let f = [f0, f1].ord_all();
```

`f0` and `f1` play at the same time, and the result finishes when
both are done.

#### 3. Any

```rust
use motiongfx::prelude::*;

let f0 = TrackFragment::new();
let f1 = TrackFragment::new();

let f = [f0, f1].ord_any();
```

`f0` and `f1` play at the same time, and the result finishes when
either one is done.

#### 4. Flow

```rust
use motiongfx::prelude::*;

let f0 = TrackFragment::new();
let f1 = TrackFragment::new();

let f = [f0, f1].ord_flow(0.5);
```

`f1` starts 0.5 seconds after `f0` begins, regardless of how long
`f0` takes.

## Officially Supported Backends

- [Bevy MotionGfx](https://crates.io/crates/bevy_motiongfx)

## Join the community!

You can join us on the [Voxell discord server](https://discord.gg/Mhnyp6VYEQ).

## Inspirations and Similar Projects

- [Motion Canvas](https://motioncanvas.io/)
- [Manim](https://www.manim.community/)

## Version Matrix

| Bevy    | MotionGfx  |
| ------- | ---------- |
| 0.18    | 0.2        |
| 0.17    | 0.1        |

## License

`motiongfx` is dual-licensed under either:

- MIT License ([LICENSE-MIT](/LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](/LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.
