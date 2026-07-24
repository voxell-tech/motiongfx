//! The scene round-trips through serde with an opaque value type.
//! Uses `f32` as a stand-in `V`; a real backend picks its own.

use core::time::Duration;

use motiongfx_scene::prelude::*;

fn sample() -> Scene<u64, f32> {
    // Chain[ All[ to(1.0), to(0.5) ], Delayed(0.2, to(2.0)) ]
    let action = |value: f32| ActionCmd {
        subject: 7,
        field: FieldRef {
            type_name: "Transform".into(),
            path: "translation::x".into(),
        },
        op: OpRef("to".into()),
        value,
        duration: Duration::from_millis(500),
        ease: Some(EaseRef::new("cubic::ease_in_out")),
        interp: None,
    };

    Scene {
        stage: Stage {
            subjects: vec![Subject { id: 7, state: 0.0 }],
        },
        animation: Block::chain(vec![
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
        ]),
    }
}

#[test]
fn scene_round_trips_through_json() {
    let scene = sample();
    let json = serde_json::to_string(&scene).unwrap();
    let back: Scene<u64, f32> = serde_json::from_str(&json).unwrap();
    assert_eq!(scene, back);
}

#[test]
fn empty_timeline_is_an_empty_chain() {
    let scene: Scene<u64, f32> = Scene {
        stage: Stage { subjects: vec![] },
        animation: Block::chain(vec![]),
    };

    assert_eq!(scene.animation.combinator, Combinator::Chain);
    assert!(scene.animation.children.is_empty());

    let json = serde_json::to_string(&scene).unwrap();
    let back: Scene<u64, f32> = serde_json::from_str(&json).unwrap();
    assert_eq!(scene, back);
}
