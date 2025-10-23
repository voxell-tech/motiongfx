# MotionGfx

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/voxell-tech/motiongfx#license)
[![Crates.io](https://img.shields.io/crates/v/motiongfx.svg)](https://crates.io/crates/motiongfx)
[![Downloads](https://img.shields.io/crates/d/motiongfx.svg)](https://crates.io/crates/motiongfx)
[![Docs](https://docs.rs/motiongfx/badge.svg)](https://docs.rs/motiongfx/latest/motiongfx/)
[![CI](https://github.com/voxell-tech/motiongfx/workflows/CI/badge.svg)](https://github.com/voxell-tech/motiongfx/actions)
[![Discord](https://img.shields.io/discord/442334985471655946.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/Mhnyp6VYEQ)

**MotionGfx** is a backend-agnostic motion graphics framework
built on top of the [Bevy](https://bevyengine.org) ECS. It provides a
modular foundation for procedural animations.

## Key Features

- **Backend agnostic**: Works with any rendering backend.
- **Procedural**: Write animations with code - loops, functions,
  logic.
- **Type-erased**: Powered by
  [Field Path](https://github.com/voxell-tech/field_path), allowing
  runtime-flexible animation of arbitrary data.
- **Two-way playback**: Play animations both forward and backward with
  no extra computation.
- **Batteries included**: Packed with common easing and interpolation
  functions.

## Core Concepts

### Timeline

`Timeline` is a top-level structure that coordinates a sequence of
tracks and their associated actions. Each track acts like a
checkpoint, allowing animations to be grouped into discrete blocks
(especially useful for creating slides).

A `Track` represents sequences of actions in chronological order, each
with a defined start time and duration. Tracks ensure that actions
within them are played in the correct temporal order.

```rust
use motiongfx::prelude::*;

// `Timeline` can only be created via a `TimelineBuilder`.
let mut b = TimelineBuilder::new();
// To create a track, you first have to create the actions.
let action = b
    // Create an action with:
    //   id   field path     action fn
    .act("x", field!(<f32>), |x| x + 1.0)
    // Every action needs an interpolation function.
    .with_interp(|&a, &b, t| a + (b - a) * t)
    // An optional easing function and be added.
    .with_ease(ease::cubic::ease_in_out);

// Once an action is created, it can be "played" into a
// `TrackFragment` with a given duration.
let frag = action.play(1.0);

// Which can then be compiled into a `Track`.
let track = frag.compile();

// 1 or more tracks can be added to the builder to create a timeline.
b.add_tracks(track);
let timeline = b.compile();
```

#### Bake and Sample Timeline

Once a timeline is created, it is ready for baking and sampling. Bake
must happen before sample. Otherwise, sampling it will be a no-op.

Registries must be created to perform baking/sampling. For more info
about registries, see below.

```rust
use motiongfx::prelude::*;

// Using a dummy world, in reality, it should be something that maps
// subjects' Ids to their animatable components.
type SubjectWorld = ();

let mut world: SubjectWorld = ();
let accessor_registry = FieldAccessorRegistry::new();
let pipeline_registry = PipelineRegistry::<SubjectWorld>::new();
let mut timeline = TimelineBuilder::new().compile();

// Bake actions into segments.
timeline.bake_actions(
    &accessor_registry,
    &pipeline_registry,
    &world,
);

// Actions needs to be queued before it can be sampled.
timeline.queue_actions();
timeline.sample_queued_actions(
    &accessor_registry,
    &pipeline_registry,
    &mut world,
);
```

### Track Ordering

`TrackFragment`s can be ordered using track ordering trait or
functions. There are 4 ways to order track fragments:

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

Chaining runs `f1` after `f0` finishes.

#### 2. All

```rust
use motiongfx::prelude::*;

let f0 = TrackFragment::new();
let f1 = TrackFragment::new();

let f = [f0, f1].ord_all();
```

All runs `f0` and `f1` concurrently and waits for all of them to
finish.

#### 3. Any

```rust
use motiongfx::prelude::*;

let f0 = TrackFragment::new();
let f1 = TrackFragment::new();

let f = [f0, f1].ord_any();
```

Any runs `f0` and `f1` concurrenly and wait for any of them to finish.

#### 4. Flow

```rust
use motiongfx::prelude::*;

let f0 = TrackFragment::new();
let f1 = TrackFragment::new();

let f = [f0, f1].ord_flow(0.5);
```

Flow runs `f1` after `f0` with a fixed delay time rather than waiting
for `f0` to finish.

### Registries

Registries are used to perform reflection and safely erase types.

#### Field Accessor Regisry

The `FieldAccessorRegistry` maintains a mapping between animatable
fields and their corresponding accessors, enabling MotionGfx to read
and write values on arbitrary data structures in a type-safe yet
dynamic way.

```rust
use motiongfx::prelude::*;

#[derive(Debug, Clone, Copy)]
struct Subject(f32);

let mut accessor_registry = FieldAccessorRegistry::new();
accessor_registry.register_typed(
    field!(<Subject>::0),
    accessor!(<Subject>::0)
);
```

#### Pipeline Registry

Pipelines handle the baking of actions and the sampling of animation
segments for playback or preview.

```rust
use std::collections::HashMap;

use motiongfx::prelude::*;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct Id(u32);
#[derive(Debug, Clone, Copy)]
struct Subject(f32);
type SubjectWorld = HashMap<Id, Subject>;

let mut pipeline_registry = PipelineRegistry::<SubjectWorld>::new();
pipeline_registry.register_unchecked(
    PipelineKey::new::<Id, Subject, f32>(),
    Pipeline::new(
        |world, ctx| {
            ctx.bake::<Id, Subject, f32>(|id| world.get(&id));
        },
        |world, ctx| {
            ctx.sample::<Id, Subject, f32>(
                |id, target, accessor| {
                    if let Some(x) = world.get_mut(&id) {
                        *accessor.get_mut(x) = target;
                    }
                },
            );
        },
    ),
);
```

### Subject World

Because MotionGfx is backend agnostic, it can be used to animate
subjects in any world. A typical subject world would hold unique Ids
that maps subject entities to their associated animatable components.

A simple example of such would be a `HashMap`.

```rust
use std::collections::HashMap;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct Id(u32);
#[derive(Debug, Clone, Copy)]
struct Subject(f32);
type SubjectWorld = HashMap<Id, Subject>;
```

Below is a comprehensive example on how MotionGfx can be used with a
custom world!

```rust
use std::collections::HashMap;

use motiongfx::prelude::*;

// First, we have to initialize a subject world and the
// registries.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct Id(u32);
#[derive(Debug, Clone, Copy)]
struct Subject(f32);
type SubjectWorld = HashMap<Id, Subject>;

let mut subject_world = SubjectWorld::new();
let mut accessor_registry = FieldAccessorRegistry::new();
let mut pipeline_registry =
    PipelineRegistry::<SubjectWorld>::new();

// The accessor registry should contain accessors to the fields in
// the subjects. In our case, it's just the first field in
// the tuple struct: `Subject::0`.

accessor_registry.register_typed(
    field!(<Subject>::0),
    accessor!(<Subject>::0),
);

// Similarly, the pipeline registry shoiud contain pipelines to
// bake and sample the fields in the subjects.

pipeline_registry.register_unchecked(
    PipelineKey::new::<Id, Subject, f32>(),
    Pipeline::new(
        |world, ctx| {
            ctx.bake::<Id, Subject, f32>(|id| world.get(&id));
        },
        |world, ctx| {
            ctx.sample::<Id, Subject, f32>(
                |id, target, accessor| {
                    if let Some(x) = world.get_mut(&id) {
                        *accessor.get_mut(x) = target;
                    }
                },
            );
        },
    ),
);

// Now that the registries are complete, we can start adding
// subjects into the subject world.

subject_world.insert(Id(1), Subject(0.0));

// A timeline can only be created via the `TimelineBuilder`.

let mut builder = TimelineBuilder::new();

let track = builder
    // Creates the action.
    .act(Id(1), field!(<Subject>::0), |x| x + 10.0)
    // Adds an interpolation method.
    .with_interp(|&a, &b, t| a + (b - a) * t)
    // Specifies the duration of the action.
    .play(1.0)
    // Compiles into a track.
    .compile();

// Adds the track to the builder.
builder.add_tracks(track);
// And compile it into a timeline.
let mut timeline = builder.compile();
// The timeline needs to be baked once before sampling can happen.
timeline.bake_actions(
    &accessor_registry,
    &pipeline_registry,
    &subject_world,
);

// Let's visualize the current state of the subject world before
// the sampling happens.
println!("Before: {:?}", subject_world);

// We fast forward the timeline.
timeline.set_target_time(0.5);
// Actions need to be queued before it can be sampled.
// The queued actions are stored internally.
timeline.queue_actions();
timeline.sample_queued_actions(
    &accessor_registry,
    &pipeline_registry,
    &mut subject_world,
);

// Visualize the state of the subject world after the sampling.
println!("After:  {:?}", subject_world);
```

## Officially Supported Backends

- [Bevy MotionGfx](https://crates.io/crates/bevy_motiongfx)

## Join the community!

You can join us on the [Voxell discord server](https://discord.gg/Mhnyp6VYEQ).

## Inspirations and Similar Projects

- [Motion Canvas](https://motioncanvas.io/)
- [Manim](https://www.manim.community/)

## License

`motiongfx` is dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.

