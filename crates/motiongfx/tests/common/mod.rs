//! Shared `World`/`Transform` fixture for integration tests: a single
//! `CUBE` subject with a lerp-able `Vec3` translation.

use std::collections::HashMap;
use std::time::Duration;

use motiongfx::prelude::*;

pub const CUBE: u32 = 0;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Transform {
    pub translation: Vec3,
}

#[derive(Default)]
pub struct World {
    pub subjects: HashMap<u32, Transform>,
}

impl SubjectSource<u32, Transform> for World {
    fn get_source(&self, id: u32) -> Option<&Transform> {
        self.subjects.get(&id)
    }

    fn apply_source<R>(
        &mut self,
        id: u32,
        f: impl FnOnce(&mut Transform) -> R,
    ) -> Option<R> {
        self.subjects.get_mut(&id).map(f)
    }
}

pub fn lerp_f32(from: &f32, to: &f32, t: f32) -> f32 {
    from + (to - from) * t
}

pub fn lerp_vec3(from: &Vec3, to: &Vec3, t: f32) -> Vec3 {
    Vec3 {
        x: lerp_f32(&from.x, &to.x, t),
        y: lerp_f32(&from.y, &to.y, t),
        z: lerp_f32(&from.z, &to.z, t),
    }
}

pub fn world_starting_at(translation: Vec3) -> World {
    World {
        subjects: HashMap::from([(CUBE, Transform { translation })]),
    }
}

pub fn sample_at(
    registry: &Registry,
    timeline: &mut Timeline<World>,
    world: &mut World,
    time: Duration,
) -> Vec3 {
    timeline.set_target_time(time);
    timeline.queue_actions();
    timeline.sample_queued_actions(registry, world);
    world.subjects[&CUBE].translation
}
