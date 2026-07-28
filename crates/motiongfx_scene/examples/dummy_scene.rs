//! Builds a small `Scene` by hand, serializes it to RON and prints it,
//! then deserializes it back and asserts it round-trips exactly.
//!
//! Run with `cargo run -p motiongfx_scene --example dummy_scene`.

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

struct ExampleBackend;

impl SceneBackend for ExampleBackend {
    type Id = u64;
    type ValuePool = ExampleValuePool;
    type OpId = Op;
    type InterpId = ();
    type EaseId = Ease;
    type World = ();
}

/// Two columns, just to show a pool holding more than one value type:
/// this scene animates a cube's scale (`f32`) and visibility (`bool`).
#[derive(
    Default, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
struct ExampleValuePool {
    f32: SparseMap<f32>,
    bool: SparseMap<bool>,
}

impl ValueColumn<f32> for ExampleValuePool {
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

impl ValueColumn<bool> for ExampleValuePool {
    fn get(&self, id: ValueId) -> Option<&bool> {
        self.bool.get(&id)
    }

    fn get_mut(&mut self, id: ValueId) -> Option<&mut bool> {
        self.bool.get_mut(&id)
    }

    fn insert(&mut self, value: bool) -> ValueId {
        self.bool.insert(value)
    }
}

fn main() {
    let mut values = ExampleValuePool::default();

    let initial_scale = values.insert(1.0_f32);
    let scale_target = values.insert(2.0_f32);
    let visible_target = values.insert(true);

    let scene = Scene {
        stage: Stage {
            subjects: vec![Subject {
                id: 0,
                state: initial_scale,
            }],
        },
        animation: Block::chain(vec![
            Node::Action(ActionCmd {
                subject: 0,
                field: FieldRef {
                    type_name: "Cube".into(),
                    path: "scale".into(),
                },
                op: Op::To,
                value: scale_target,
                duration: Duration::from_millis(600),
                ease: Some(Ease::CubicEaseInOut),
                interp: None,
            }),
            Node::Action(ActionCmd {
                subject: 0,
                field: FieldRef {
                    type_name: "Cube".into(),
                    path: "visible".into(),
                },
                op: Op::To,
                value: visible_target,
                duration: Duration::ZERO,
                ease: None,
                interp: None,
            }),
        ]),
        values,
    };

    let ron_text = ron::ser::to_string_pretty(
        &scene,
        ron::ser::PrettyConfig::default(),
    )
    .expect("scene should serialize");
    println!("{ron_text}");

    let back: Scene<ExampleBackend> = ron::de::from_str(&ron_text)
        .expect("scene should deserialize");
    assert_eq!(scene, back);
}
