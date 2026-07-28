//! The scene round-trips through serde with an open value pool.
//! Uses a single `f32` column as a stand-in `ValuePool`; a real
//! backend picks whatever columns it needs.

use core::time::Duration;

use motiongfx_scene::prelude::*;
use serde::{Deserialize, Serialize};
use sparse_map::SparseMap;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
enum Op {
    To,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
enum Ease {
    CubicEaseInOut,
}

struct RoundtripBackend;

impl SceneBackend for RoundtripBackend {
    type Id = u64;
    type ValuePool = RoundtripValuePool;
    type OpId = Op;
    type InterpId = ();
    type EaseId = Ease;
    type World = ();
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct RoundtripValuePool {
    f32: SparseMap<f32>,
}

impl ValueColumn<f32> for RoundtripValuePool {
    fn get(&self, id: ValueId) -> Option<&f32> {
        self.f32.get(&id)
    }

    fn get_mut(&mut self, id: ValueId) -> Option<&mut f32> {
        self.f32.get_mut(&id)
    }

    fn insert(&mut self, value: f32) -> ValueId {
        self.f32.insert(value)
    }
}

fn sample() -> Scene<RoundtripBackend> {
    // Chain[ All[ to(1.0), to(0.5) ], Delayed(0.2, to(2.0)) ]
    let mut values = RoundtripValuePool::default();
    let mut action = |value: f32| ActionCmd {
        subject: 7,
        field: FieldRef {
            type_name: "Transform".into(),
            path: "translation::x".into(),
        },
        op: Op::To,
        value: values.insert(value),
        duration: Duration::from_millis(500),
        ease: Some(Ease::CubicEaseInOut),
        interp: None,
    };

    let animation = Block::chain(vec![
        Node::Block(Block {
            combinator: Combinator::All,
            children: vec![
                Node::Action(action(1.0)),
                Node::Action(action(0.5)),
            ],
        }),
        Node::Delayed {
            offset: Duration::from_millis(200),
            node: Box::new(Node::Action(action(2.0))),
        },
    ]);
    let state = values.insert(0.0);

    Scene {
        stage: Stage {
            subjects: vec![Subject { id: 7, state }],
        },
        animation,
        values,
    }
}

#[test]
fn scene_round_trips_through_json() {
    let scene = sample();
    let json = serde_json::to_string(&scene).unwrap();
    let back: Scene<RoundtripBackend> =
        serde_json::from_str(&json).unwrap();
    assert_eq!(scene, back);
}

#[test]
fn empty_timeline_is_an_empty_chain() {
    let scene: Scene<RoundtripBackend> = Scene {
        stage: Stage { subjects: vec![] },
        animation: Block::chain(vec![]),
        values: RoundtripValuePool::default(),
    };

    assert_eq!(scene.animation.combinator, Combinator::Chain);
    assert!(scene.animation.children.is_empty());

    let json = serde_json::to_string(&scene).unwrap();
    let back: Scene<RoundtripBackend> =
        serde_json::from_str(&json).unwrap();
    assert_eq!(scene, back);
}
