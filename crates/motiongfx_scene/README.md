# MotionGfx Scene

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/voxell-tech/motiongfx#license)
[![Crates.io](https://img.shields.io/crates/v/motiongfx_scene.svg)](https://crates.io/crates/motiongfx_scene)
[![Downloads](https://img.shields.io/crates/d/motiongfx_scene.svg)](https://crates.io/crates/motiongfx_scene)
[![Docs](https://docs.rs/motiongfx_scene/badge.svg)](https://docs.rs/motiongfx_scene/latest/motiongfx_scene/)
[![CI](https://github.com/voxell-tech/motiongfx/workflows/CI/badge.svg)](https://github.com/voxell-tech/motiongfx/actions)
[![Discord](https://img.shields.io/discord/442334985471655946.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/Mhnyp6VYEQ)

**MotionGfx Scene** is a serializable scene and animation document for
[MotionGfx](https://crates.io/crates/motiongfx).

A scene never names a concrete type. It refers to fields, ops, and
values *by name*. A `SceneBackend` supplies the id and pool types, and
a `SceneRegistry` supplies the code that turns those names back into
typed accessors and `motiongfx` actions. Three layers:

## 1. Format

Pure data, no engine and no backend. A `Scene<B>` is three parts:

- **`stage`**: The value every animated field starts from. One
  `FieldSeed` (a `FieldRef` plus a value id) per subject field.
- **`animation`**: A tree of `Block`s. Each block has a `Combinator`
  (`Chain`, `All`, `Flow`) and children that are nested blocks, leaf
  `Node::action(ActionCmd)`s, or `Node::draft` timing slots. A `Node`
  can carry a `delay`.
- **`values`**: The `ValuePool`. Every number an action or seed refers
  to, addressed by id so the tree itself stays plain data.

The examples below use a toy backend with one `f32` column and a
`Point` world, defined in
[`docs/backend.rs`](docs/backend.rs).

```rust
# #[path = "docs/backend.rs"] mod _doc; use _doc::*;
let mut values = Pool::default();

let start = values.insert(0.0); // Point 0 starts at x = 0.0
let target = values.insert(5.0); // and is driven to x = 5.0

let scene: Scene<Toy> = Scene {
    stage: Stage {
        subjects: vec![Subject {
            id: 0,
            fields: vec![FieldSeed { field: field("x"), value: start }],
        }],
    },
    animation: Block::chain(vec![Node::action(ActionCmd {
        subject: 0,
        field: field("x"),
        op: Op::To,
        value: target,
        duration: ms(200),
        ease: None,
        interp: None,
        name: None,
    })]),
    values,
};
```

Because it is all data, a scene round-trips through serde:

```rust
# #[path = "docs/backend.rs"] mod _doc; use _doc::*;
let scene = scene();
let ron = ron::ser::to_string(&scene).unwrap();
let back: Scene<Toy> = ron::de::from_str(&ron).unwrap();
assert_eq!(scene, back);
```

## 2. Registry

`SceneRegistry<B>` is where the app teaches the format its types,
before any scene is compiled. It binds names to code:

- **`register_field`**: Binds a `FieldRef` (owner `"Point"`, path
  `"::x"`) to a typed `FieldAccessor`.
- **`register_op`**: Binds an `OpId` to a closure that builds a
  `motiongfx` `Action<T>`.

```rust
# #[path = "docs/backend.rs"] mod _doc; use _doc::*;
let mut registry: SceneRegistry<Toy> = SceneRegistry::new();

// "Point" + "::x" now resolves to a real accessor.
registry.register_field::<Point, f32>("Point".into(), path!(<Point>::x));

// `Op::To` builds an action that ignores the previous value.
registry.register_op::<f32, _>(Op::To, |value: &f32| {
    let value = *value;
    Box::new(move |_prev: &f32| value) as Box<dyn Action<f32>>
});
```

Op registration is keyed by the *value* type, not the owning type: one
`register_op::<f32, _>(Op::To, ..)` serves `Point::x`, `Circle::radius`,
and every other `f32` field.

## 3. Runtime

`Scene::compile` walks the block tree, resolves every `FieldRef` and
`OpId` through the registry, and folds the blocks by their combinators
into a `motiongfx::Timeline`. `Scene::stage` writes the seed values
straight into the world - what `Timeline::bake_actions` then reads as
each track's starting point.

```rust
# #[path = "docs/backend.rs"] mod _doc; use _doc::*;
let scene = scene();
let scene_registry = registry(); // fields + the `To` op

let mut runtime = Registry::new();
let mut timeline = scene.compile(&scene_registry, &mut runtime).unwrap();

let mut world = World::default();
world.points.insert(0, Point::default());

// Seed the world, then bake the starting values off it.
scene.stage(&scene_registry, &mut world).unwrap();
timeline.bake_actions(&runtime, &world);

// Sample at t = 200ms: the action has run to completion.
timeline.set_target_time(ms(200));
timeline.queue_actions();
timeline.sample_queued_actions(&runtime, &mut world);
assert_eq!(world.points[&0].x, 5.0);
```

Resolution is fallible: an unregistered field or op is a
`CompileError`, never a panic.

## Join the community!

You can join us on the [Voxell discord server](https://discord.gg/Mhnyp6VYEQ).

## License

`motiongfx_scene` is dual-licensed under either:

- MIT License ([LICENSE-MIT](/LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](/LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.
