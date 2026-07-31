//! End-to-end tests of `compile()`: scene -> registry -> `Timeline`,
//! baked and sampled against a toy [`SubjectSource`] world.

use std::collections::HashMap;
use std::time::Duration;

use motiongfx::action::Action;
use motiongfx::prelude::*;
use motiongfx_scene::compile::{apply_stage, compile};
use motiongfx_scene::prelude::*;
use motiongfx_scene::registry::SceneRegistry;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
enum Op {
    To,
}

struct ToyBackend;

impl SceneBackend for ToyBackend {
    type Id = u64;
    type ValueId = Uuid;
    type ValuePool = ToyValuePool;
    type OpId = Op;
    type InterpId = ();
    type EaseId = ();
    type World = ToyWorld;
}

/// A single `f32` column: every value in these tests is a plain
/// `f32`, so one map is the whole pool.
#[derive(
    Default, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
struct ToyValuePool {
    f32: HashMap<Uuid, f32>,
}

impl ValueColumn<Uuid, f32> for ToyValuePool {
    fn get(&self, id: Uuid) -> Option<&f32> {
        self.f32.get(&id)
    }

    fn get_mut(&mut self, id: Uuid) -> Option<&mut f32> {
        self.f32.get_mut(&id)
    }

    fn insert(&mut self, value: f32) -> Uuid {
        let id = Uuid::new_v4();
        self.f32.insert(id, value);
        id
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Point {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct Circle {
    radius: f32,
}

#[derive(Default)]
struct ToyWorld {
    points: HashMap<u64, Point>,
    circles: HashMap<u64, Circle>,
}

impl SubjectSource<u64, Point> for ToyWorld {
    fn get_source(&self, id: u64) -> Option<&Point> {
        self.points.get(&id)
    }

    fn apply_source<R>(
        &mut self,
        id: u64,
        f: impl FnOnce(&mut Point) -> R,
    ) -> Option<R> {
        self.points.get_mut(&id).map(f)
    }
}

impl SubjectSource<u64, Circle> for ToyWorld {
    fn get_source(&self, id: u64) -> Option<&Circle> {
        self.circles.get(&id)
    }

    fn apply_source<R>(
        &mut self,
        id: u64,
        f: impl FnOnce(&mut Circle) -> R,
    ) -> Option<R> {
        self.circles.get_mut(&id).map(f)
    }
}

fn field(path: &str) -> FieldRef {
    FieldRef::new("Point", format!("::{path}"))
}

fn action_cmd(
    values: &mut ToyValuePool,
    subject: u64,
    path: &str,
    value: f32,
    duration_ms: u64,
) -> ActionCmd<ToyBackend> {
    ActionCmd {
        subject,
        field: field(path),
        op: Op::To,
        value: values.insert(value),
        duration: ms(duration_ms),
        ease: None,
        interp: None,
    }
}

/// A subject whose `paths` all start at `0.0` - the initial value
/// baking would otherwise read off the world.
fn subject(
    values: &mut ToyValuePool,
    id: u64,
    paths: &[&str],
) -> Subject<ToyBackend> {
    let fields = paths
        .iter()
        .map(|path| FieldSeed {
            field: field(path),
            value: values.insert(0.0),
        })
        .collect();
    Subject { id, fields }
}

/// Ignores the previous value; one op registration serves every
/// `to(value)` command.
fn registry_with_to_op() -> SceneRegistry<ToyBackend> {
    let mut registry = SceneRegistry::new();
    registry.register_field::<Point, f32>(
        "Point".into(),
        path!(<Point>::x),
    );
    registry.register_field::<Point, f32>(
        "Point".into(),
        path!(<Point>::y),
    );
    registry.register_op::<f32, _>(
        Op::To,
        |value: &f32| -> Box<dyn Action<f32>> {
            let value = *value;
            Box::new(move |_prev: &f32| value)
        },
    );
    registry
}

fn sample_at(
    timeline: &mut Timeline<ToyWorld>,
    registry: &Registry,
    world: &mut ToyWorld,
    at: Duration,
) {
    timeline.set_target_time(at);
    timeline.queue_actions();
    timeline.sample_queued_actions(registry, world);
}

#[test]
fn apply_stage_writes_initial_values_into_the_world() {
    let scene_registry = registry_with_to_op();
    let mut values = ToyValuePool::default();
    let staged = values.insert(7.0);
    let scene: Scene<ToyBackend> = Scene {
        stage: Stage {
            subjects: vec![Subject {
                id: 0,
                fields: vec![FieldSeed {
                    field: field("x"),
                    value: staged,
                }],
            }],
        },
        animation: Block::chain(vec![]),
        values,
    };

    let mut world = ToyWorld::default();
    world.points.insert(0, Point::default());
    assert_eq!(world.points[&0].x, 0.0);

    apply_stage(&scene, &scene_registry, &mut world)
        .expect("stage should apply");

    // What `bake_actions` would then read as the track's start.
    assert_eq!(world.points[&0].x, 7.0);
}

#[test]
fn apply_stage_rejects_an_unregistered_field() {
    let scene_registry: SceneRegistry<ToyBackend> =
        SceneRegistry::new();
    let mut values = ToyValuePool::default();
    let staged = values.insert(7.0);
    let scene: Scene<ToyBackend> = Scene {
        stage: Stage {
            subjects: vec![Subject {
                id: 0,
                fields: vec![FieldSeed {
                    field: field("x"),
                    value: staged,
                }],
            }],
        },
        animation: Block::chain(vec![]),
        values,
    };

    let mut world = ToyWorld::default();
    world.points.insert(0, Point::default());

    let err = apply_stage(&scene, &scene_registry, &mut world)
        .expect_err("unregistered field should not resolve");
    assert!(matches!(err, CompileError::UnknownField(_)));
}

#[test]
fn compiles_and_samples_a_single_action() {
    let scene_registry = registry_with_to_op();
    let mut values = ToyValuePool::default();
    let scene: Scene<ToyBackend> = Scene {
        stage: Stage {
            subjects: vec![subject(&mut values, 0, &["x"])],
        },
        animation: Block::chain(vec![Node::action(action_cmd(
            &mut values,
            0,
            "x",
            5.0,
            200,
        ))]),
        values,
    };

    let mut runtime_registry = Registry::new();
    let mut timeline =
        compile(&scene, &scene_registry, &mut runtime_registry)
            .expect("scene should compile");

    let mut world = ToyWorld::default();
    world.points.insert(0, Point::default());
    timeline.bake_actions(&runtime_registry, &world);

    sample_at(&mut timeline, &runtime_registry, &mut world, ms(200));
    assert_eq!(world.points[&0].x, 5.0);
}

#[test]
fn chain_runs_children_in_sequence() {
    let scene_registry = registry_with_to_op();
    let mut values = ToyValuePool::default();
    let scene: Scene<ToyBackend> = Scene {
        stage: Stage {
            subjects: vec![subject(&mut values, 0, &["x"])],
        },
        animation: Block::chain(vec![
            Node::action(action_cmd(&mut values, 0, "x", 1.0, 100)),
            Node::action(action_cmd(&mut values, 0, "x", 2.0, 100)),
        ]),
        values,
    };

    let mut runtime_registry = Registry::new();
    let mut timeline =
        compile(&scene, &scene_registry, &mut runtime_registry)
            .expect("scene should compile");

    let mut world = ToyWorld::default();
    world.points.insert(0, Point::default());
    timeline.bake_actions(&runtime_registry, &world);

    // Only the first action's window has elapsed.
    sample_at(&mut timeline, &runtime_registry, &mut world, ms(100));
    assert_eq!(world.points[&0].x, 1.0);

    // Both actions have now completed, in order.
    sample_at(&mut timeline, &runtime_registry, &mut world, ms(200));
    assert_eq!(world.points[&0].x, 2.0);
}

#[test]
fn all_combinator_runs_children_simultaneously() {
    let scene_registry = registry_with_to_op();
    let mut values = ToyValuePool::default();
    let scene: Scene<ToyBackend> = Scene {
        stage: Stage {
            subjects: vec![subject(&mut values, 0, &["x", "y"])],
        },
        animation: Block {
            combinator: Combinator::All,
            children: vec![
                Node::action(action_cmd(
                    &mut values,
                    0,
                    "x",
                    5.0,
                    200,
                )),
                Node::action(action_cmd(
                    &mut values,
                    0,
                    "y",
                    9.0,
                    200,
                )),
            ],
        },
        values,
    };

    let mut runtime_registry = Registry::new();
    let mut timeline =
        compile(&scene, &scene_registry, &mut runtime_registry)
            .expect("scene should compile");

    let mut world = ToyWorld::default();
    world.points.insert(0, Point::default());
    timeline.bake_actions(&runtime_registry, &world);

    sample_at(&mut timeline, &runtime_registry, &mut world, ms(200));
    assert_eq!(world.points[&0].x, 5.0);
    assert_eq!(world.points[&0].y, 9.0);
}

#[test]
fn delayed_node_shifts_the_start_time() {
    let scene_registry = registry_with_to_op();
    let mut values = ToyValuePool::default();
    let scene: Scene<ToyBackend> = Scene {
        stage: Stage {
            subjects: vec![subject(&mut values, 0, &["x"])],
        },
        animation: Block::chain(vec![
            Node::action(action_cmd(&mut values, 0, "x", 5.0, 100))
                .delay(ms(200)),
        ]),
        values,
    };

    let mut runtime_registry = Registry::new();
    let mut timeline =
        compile(&scene, &scene_registry, &mut runtime_registry)
            .expect("scene should compile");

    let mut world = ToyWorld::default();
    world.points.insert(0, Point::default());
    timeline.bake_actions(&runtime_registry, &world);

    // Still inside the delay window: nothing has been queued yet.
    sample_at(&mut timeline, &runtime_registry, &mut world, ms(100));
    assert_eq!(world.points[&0].x, 0.0);

    // Delay elapsed and the action's own duration has passed.
    sample_at(&mut timeline, &runtime_registry, &mut world, ms(300));
    assert_eq!(world.points[&0].x, 5.0);
}

#[test]
fn one_op_registration_covers_every_owning_type_sharing_t() {
    let mut scene_registry: SceneRegistry<ToyBackend> =
        SceneRegistry::new();
    scene_registry.register_field::<Point, f32>(
        "Point".into(),
        path!(<Point>::x),
    );
    scene_registry.register_field::<Circle, f32>(
        "Circle".into(),
        path!(<Circle>::radius),
    );
    // Registered once, for T = f32; never registered again for Circle.
    scene_registry.register_op::<f32, _>(
        Op::To,
        |value: &f32| -> Box<dyn Action<f32>> {
            let value = *value;
            Box::new(move |_prev: &f32| value)
        },
    );

    let mut values = ToyValuePool::default();
    let circle_action = ActionCmd {
        subject: 0,
        field: FieldRef::new("Circle", "::radius"),
        op: Op::To,
        value: values.insert(3.0),
        duration: ms(100),
        ease: None,
        interp: None,
    };
    // Two owning types on one subject, so two initial entries.
    let mut staged = subject(&mut values, 0, &["x"]);
    staged.fields.push(FieldSeed {
        field: FieldRef::new("Circle", "::radius"),
        value: values.insert(0.0),
    });
    let scene: Scene<ToyBackend> = Scene {
        stage: Stage {
            subjects: vec![staged],
        },
        animation: Block::chain(vec![
            Node::action(action_cmd(&mut values, 0, "x", 5.0, 100)),
            Node::action(circle_action),
        ]),
        values,
    };

    let mut runtime_registry = Registry::new();
    let mut timeline =
        compile(&scene, &scene_registry, &mut runtime_registry)
            .expect("scene should compile");

    let mut world = ToyWorld::default();
    world.points.insert(0, Point::default());
    world.circles.insert(0, Circle::default());
    timeline.bake_actions(&runtime_registry, &world);

    sample_at(&mut timeline, &runtime_registry, &mut world, ms(200));
    assert_eq!(world.points[&0].x, 5.0);
    assert_eq!(world.circles[&0].radius, 3.0);
}

#[test]
fn unregistered_field_is_a_compile_error() {
    // No `register_field` call for "x" at all: `resolve_op` dispatches
    // on the field first, so this never reaches op resolution.
    let scene_registry: SceneRegistry<ToyBackend> =
        SceneRegistry::new();

    let mut values = ToyValuePool::default();
    let scene: Scene<ToyBackend> = Scene {
        stage: Stage {
            subjects: vec![subject(&mut values, 0, &["x"])],
        },
        animation: Block::chain(vec![Node::action(action_cmd(
            &mut values,
            0,
            "x",
            5.0,
            100,
        ))]),
        values,
    };

    let mut runtime_registry = Registry::new();
    let err =
        match compile(&scene, &scene_registry, &mut runtime_registry)
        {
            Err(err) => err,
            Ok(_) => {
                panic!("unregistered field must fail to compile")
            }
        };
    assert!(matches!(err, CompileError::UnknownField(_)));
}

#[test]
fn unregistered_op_is_a_compile_error() {
    // Field is registered, but no op is.
    let mut scene_registry: SceneRegistry<ToyBackend> =
        SceneRegistry::new();
    scene_registry.register_field::<Point, f32>(
        "Point".into(),
        path!(<Point>::x),
    );

    let mut values = ToyValuePool::default();
    let scene: Scene<ToyBackend> = Scene {
        stage: Stage {
            subjects: vec![subject(&mut values, 0, &["x"])],
        },
        animation: Block::chain(vec![Node::action(action_cmd(
            &mut values,
            0,
            "x",
            5.0,
            100,
        ))]),
        values,
    };

    let mut runtime_registry = Registry::new();
    let err =
        match compile(&scene, &scene_registry, &mut runtime_registry)
        {
            Err(err) => err,
            Ok(_) => panic!("unregistered op must fail to compile"),
        };
    assert!(matches!(err, CompileError::UnknownOp(_, _)));
}
