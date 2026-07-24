//! End-to-end tests of `compile()`: scene -> registry -> `Timeline`,
//! baked and sampled against a toy [`SubjectSource`] world.

use std::collections::HashMap;
use std::time::Duration;

use motiongfx::action::Action;
use motiongfx::prelude::*;
use motiongfx_scene::compile::compile;
use motiongfx_scene::prelude::*;
use motiongfx_scene::registry::SceneRegistry;

#[derive(Debug, Clone, Copy, Default)]
struct Point {
    x: f32,
    y: f32,
}

#[derive(Default)]
struct ToyWorld(HashMap<u64, Point>);

impl SubjectSource<u64, Point> for ToyWorld {
    fn get_source(&self, id: u64) -> Option<&Point> {
        self.0.get(&id)
    }

    fn apply_source<R>(
        &mut self,
        id: u64,
        f: impl FnOnce(&mut Point) -> R,
    ) -> Option<R> {
        self.0.get_mut(&id).map(f)
    }
}

fn field(path: &str) -> FieldRef {
    FieldRef {
        type_name: "Point".into(),
        path: path.into(),
    }
}

fn action_cmd(
    subject: u64,
    path: &str,
    value: f32,
    duration_ms: u64,
) -> ActionCmd<u64, f32> {
    ActionCmd {
        subject,
        field: field(path),
        op: OpRef("to".into()),
        value,
        duration: Duration::from_millis(duration_ms),
        ease: None,
        interp: None,
    }
}

/// Ignores the previous value; one op registration serves every
/// `to(value)` command.
fn registry_with_to_op() -> SceneRegistry<u64, f32, ToyWorld> {
    let mut registry = SceneRegistry::new();
    registry.register_field::<Point, f32>(
        "Point".into(),
        "x",
        path!(<Point>::x),
    );
    registry.register_field::<Point, f32>(
        "Point".into(),
        "y",
        path!(<Point>::y),
    );
    registry.register_op::<Point, f32, _>(
        "Point".into(),
        OpRef("to".into()),
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
fn compiles_and_samples_a_single_action() {
    let scene_registry = registry_with_to_op();
    let scene: Scene<u64, f32> = Scene {
        stage: Stage {
            subjects: vec![Subject { id: 0, state: 0.0 }],
        },
        animation: Block::chain(vec![Node::Action(action_cmd(
            0, "x", 5.0, 200,
        ))]),
    };

    let mut runtime_registry = Registry::new();
    let mut timeline =
        compile(&scene, &scene_registry, &mut runtime_registry)
            .expect("scene should compile");

    let mut world = ToyWorld::default();
    world.0.insert(0, Point::default());
    timeline.bake_actions(&runtime_registry, &world);

    sample_at(
        &mut timeline,
        &runtime_registry,
        &mut world,
        Duration::from_millis(200),
    );
    assert_eq!(world.0[&0].x, 5.0);
}

#[test]
fn chain_runs_children_in_sequence() {
    let scene_registry = registry_with_to_op();
    let scene: Scene<u64, f32> = Scene {
        stage: Stage {
            subjects: vec![Subject { id: 0, state: 0.0 }],
        },
        animation: Block::chain(vec![
            Node::Action(action_cmd(0, "x", 1.0, 100)),
            Node::Action(action_cmd(0, "x", 2.0, 100)),
        ]),
    };

    let mut runtime_registry = Registry::new();
    let mut timeline =
        compile(&scene, &scene_registry, &mut runtime_registry)
            .expect("scene should compile");

    let mut world = ToyWorld::default();
    world.0.insert(0, Point::default());
    timeline.bake_actions(&runtime_registry, &world);

    // Only the first action's window has elapsed.
    sample_at(
        &mut timeline,
        &runtime_registry,
        &mut world,
        Duration::from_millis(100),
    );
    assert_eq!(world.0[&0].x, 1.0);

    // Both actions have now completed, in order.
    sample_at(
        &mut timeline,
        &runtime_registry,
        &mut world,
        Duration::from_millis(200),
    );
    assert_eq!(world.0[&0].x, 2.0);
}

#[test]
fn all_combinator_runs_children_simultaneously() {
    let scene_registry = registry_with_to_op();
    let scene: Scene<u64, f32> = Scene {
        stage: Stage {
            subjects: vec![Subject { id: 0, state: 0.0 }],
        },
        animation: Block {
            combinator: Combinator::All,
            children: vec![
                Node::Action(action_cmd(0, "x", 5.0, 200)),
                Node::Action(action_cmd(0, "y", 9.0, 200)),
            ],
        },
    };

    let mut runtime_registry = Registry::new();
    let mut timeline =
        compile(&scene, &scene_registry, &mut runtime_registry)
            .expect("scene should compile");

    let mut world = ToyWorld::default();
    world.0.insert(0, Point::default());
    timeline.bake_actions(&runtime_registry, &world);

    sample_at(
        &mut timeline,
        &runtime_registry,
        &mut world,
        Duration::from_millis(200),
    );
    assert_eq!(world.0[&0].x, 5.0);
    assert_eq!(world.0[&0].y, 9.0);
}

#[test]
fn delayed_node_shifts_the_start_time() {
    let scene_registry = registry_with_to_op();
    let scene: Scene<u64, f32> = Scene {
        stage: Stage {
            subjects: vec![Subject { id: 0, state: 0.0 }],
        },
        animation: Block::chain(vec![Node::Delayed {
            offset: Duration::from_millis(200),
            node: Box::new(Node::Action(action_cmd(
                0, "x", 5.0, 100,
            ))),
        }]),
    };

    let mut runtime_registry = Registry::new();
    let mut timeline =
        compile(&scene, &scene_registry, &mut runtime_registry)
            .expect("scene should compile");

    let mut world = ToyWorld::default();
    world.0.insert(0, Point::default());
    timeline.bake_actions(&runtime_registry, &world);

    // Still inside the delay window: nothing has been queued yet.
    sample_at(
        &mut timeline,
        &runtime_registry,
        &mut world,
        Duration::from_millis(100),
    );
    assert_eq!(world.0[&0].x, 0.0);

    // Delay elapsed and the action's own duration has passed.
    sample_at(
        &mut timeline,
        &runtime_registry,
        &mut world,
        Duration::from_millis(300),
    );
    assert_eq!(world.0[&0].x, 5.0);
}

#[test]
fn unregistered_field_is_a_compile_error() {
    // Op is registered, but no `register_field` call for "x": isolates
    // the field lookup, since op lookup happens first in `resolve_op`.
    let mut scene_registry: SceneRegistry<u64, f32, ToyWorld> =
        SceneRegistry::new();
    scene_registry.register_op::<Point, f32, _>(
        "Point".into(),
        OpRef("to".into()),
        |value: &f32| -> Box<dyn Action<f32>> {
            let value = *value;
            Box::new(move |_prev: &f32| value)
        },
    );

    let scene: Scene<u64, f32> = Scene {
        stage: Stage {
            subjects: vec![Subject { id: 0, state: 0.0 }],
        },
        animation: Block::chain(vec![Node::Action(action_cmd(
            0, "x", 5.0, 100,
        ))]),
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
    let mut scene_registry: SceneRegistry<u64, f32, ToyWorld> =
        SceneRegistry::new();
    scene_registry.register_field::<Point, f32>(
        "Point".into(),
        "x",
        path!(<Point>::x),
    );

    let scene: Scene<u64, f32> = Scene {
        stage: Stage {
            subjects: vec![Subject { id: 0, state: 0.0 }],
        },
        animation: Block::chain(vec![Node::Action(action_cmd(
            0, "x", 5.0, 100,
        ))]),
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
