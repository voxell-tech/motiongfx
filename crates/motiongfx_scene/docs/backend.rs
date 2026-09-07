//! Shared setup for the crate-level doc examples: a toy `SceneBackend`
//! with a single `f32` value column, `u64` subject ids, and a `Point`
//! world.

#![allow(dead_code)]

pub use motiongfx::action::Action;
pub use motiongfx::prelude::*;
pub use motiongfx_scene::prelude::*;
pub use motiongfx_scene::registry::SceneRegistry;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// The one op these examples use: overwrite the field with the value.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum Op {
    To,
}

pub struct Toy;

impl SceneBackend for Toy {
    type Id = u64;
    type ValueId = u64;
    type ValuePool = Pool;
    type OpId = Op;
    type InterpId = ();
    type EaseId = ();
    type World = World;
}

/// One `f32` column, keyed by a running id.
#[derive(
    Default, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
pub struct Pool {
    next: u64,
    f32: HashMap<u64, f32>,
}

impl ValueColumn<u64, f32> for Pool {
    fn get(&self, id: u64) -> Option<&f32> {
        self.f32.get(&id)
    }

    fn get_mut(&mut self, id: u64) -> Option<&mut f32> {
        self.f32.get_mut(&id)
    }

    fn insert(&mut self, value: f32) -> u64 {
        let id = self.next;
        self.next += 1;
        self.f32.insert(id, value);
        id
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Default)]
pub struct World {
    pub points: HashMap<u64, Point>,
}

impl SubjectSource<u64, Point> for World {
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

/// A registry that knows `Point::x` / `Point::y` and the `To` op.
pub fn registry() -> SceneRegistry<Toy> {
    let mut r = SceneRegistry::new();
    r.register_field::<Point, f32>("Point".into(), path!(<Point>::x));
    r.register_field::<Point, f32>("Point".into(), path!(<Point>::y));
    r.register_op::<f32, _>(
        Op::To,
        |value: &f32| -> Box<dyn Action<f32>> {
            let value = *value;
            Box::new(move |_prev: &f32| value)
        },
    );
    r
}

/// `Point::<path>` as a [`FieldRef`].
pub fn field(path: &str) -> FieldRef {
    FieldRef::new("Point", format!("::{path}"))
}

/// Subject 0 starts at `x = 0.0`; one action drives it to `5.0` over
/// 200ms.
pub fn scene() -> Scene<Toy> {
    let mut values = Pool::default();
    let start = values.insert(0.0);
    let target = values.insert(5.0);

    Scene {
        stage: Stage {
            subjects: vec![Subject {
                id: 0,
                fields: vec![FieldSeed {
                    field: field("x"),
                    value: start,
                }],
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
    }
}
