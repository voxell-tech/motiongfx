//! Builds a small `Scene` by hand, serializes it to RON and prints it,
//! then deserializes it back and asserts it round-trips exactly.
//!
//! Run with `cargo run -p motiongfx_scene --example dummy_scene`.

use core::time::Duration;

use hashbrown::HashMap;
use motiongfx::prelude::*;
use motiongfx_scene::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn main() {
    let mut values = ExampleValuePool::default();

    let initial_scale = values.insert(1.0_f32);
    let initial_visible = values.insert(false);
    let scale_target = values.insert(2.0_f32);
    let visible_target = values.insert(true);

    let scene = Scene {
        stage: Stage {
            subjects: vec![Subject {
                id: 0,
                fields: vec![
                    FieldSeed {
                        field: FieldRef::new("Cube", "scale"),
                        value: initial_scale,
                    },
                    FieldSeed {
                        field: FieldRef::new("Cube", "visible"),
                        value: initial_visible,
                    },
                ],
            }],
        },
        animation: Block::chain(vec![
            Node::action(ActionCmd {
                subject: 0,
                field: FieldRef::new("Cube", "scale"),
                op: Op::To,
                value: scale_target,
                duration: ms(600),
                ease: Some(Ease::CubicEaseInOut),
                interp: None,
                name: None,
            }),
            Node::action(ActionCmd {
                subject: 0,
                field: FieldRef::new("Cube", "visible"),
                op: Op::To,
                value: visible_target,
                duration: Duration::ZERO,
                ease: None,
                interp: None,
                name: None,
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
    type ValueId = Uuid;
    type ValuePool = ExampleValuePool;
    type OpId = Op;
    type InterpId = ();
    type EaseId = Ease;
    type World = ();
}

/// Two columns, to show a pool holding more than one value type.
#[derive(
    Default, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
struct ExampleValuePool {
    f32: HashMap<Uuid, f32>,
    bool: HashMap<Uuid, bool>,
}

impl ValueColumn<Uuid, f32> for ExampleValuePool {
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

impl ValueColumn<Uuid, bool> for ExampleValuePool {
    fn get(&self, id: Uuid) -> Option<&bool> {
        self.bool.get(&id)
    }

    fn get_mut(&mut self, id: Uuid) -> Option<&mut bool> {
        self.bool.get_mut(&id)
    }

    fn insert(&mut self, value: bool) -> Uuid {
        let id = Uuid::new_v4();
        self.bool.insert(id, value);
        id
    }
}
