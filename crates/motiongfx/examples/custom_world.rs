use std::collections::HashMap;

use motiongfx::prelude::*;

struct World {
    registry: Registry,
    subject_world: SubjectWorld,
}

impl World {
    pub fn new() -> Self {
        Self {
            registry: Registry::new(),
            subject_world: SubjectWorld {
                world: HashMap::new(),
            },
        }
    }
}

// Ideally, the world should be able to erase the subject types more
// efficiently (like the Bevy's ECS world).
#[derive(Debug)]
struct SubjectWorld {
    world: HashMap<Id, Subject>,
}

#[derive(
    Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy,
)]
struct Id(u64);

#[derive(Debug, Clone, Copy)]
enum Subject {
    Point(Point),
    Line(Line),
}

#[derive(Debug, Default, Clone, Copy)]
struct Point {
    x: f32,
    y: f32,
}

#[derive(Debug, Default, Clone, Copy)]
struct Line {
    p0: Point,
    p1: Point,
}

fn main() {
    let mut world = World::new();

    // Spawn in some subjects.
    world
        .subject_world
        .world
        .insert(Id(0), Subject::Point(Point::default()));
    world
        .subject_world
        .world
        .insert(Id(1), Subject::Line(Line::default()));

    let mut builder = world.registry.create_builder();

    // Create the track.
    let track = [
        builder
            .act(Id(0), path!(<Point>::x), |x| x + 72.0)
            .with_interp(linear_f32)
            .play(1.0),
        [
            builder
                .act(Id(1), path!(<Line>::p0::y), |y| y + 42.0)
                .with_interp(linear_f32)
                .play(2.0),
            builder
                .act(Id(1), path!(<Line>::p1), |_| Point {
                    x: 6.0,
                    y: 6.0,
                })
                .with_interp(linear_point)
                .play(2.0),
        ]
        .ord_all(),
    ]
    .ord_chain();

    // Compile into a timeline.
    builder.add_tracks(track.compile());
    let mut timeline = builder.compile();

    // Bake actions into segments.
    timeline.bake_actions(&world.registry, &world.subject_world);

    // Change the target time.
    timeline.set_target_time(1.5);

    // Check the values before sampling:
    println!("Before: {:?}", world.subject_world);

    // Queue and sample the actions.
    timeline.queue_actions();
    timeline.sample_queued_actions(
        &world.registry,
        &mut world.subject_world,
    );

    // Check the values after sampling:
    println!("After:  {:?}\n", world.subject_world);

    let new_target_time = 7.0;

    // Set target time to after total track duration
    timeline.set_target_time(new_target_time);

    println!("timeline target time set to: {}s", new_target_time);

    println!(
        "# Before sampling \ncurrent time: {}s,\ntarget time: {}s",
        timeline.curr_time(),
        timeline.target_time()
    );
    println!("target time clamped to timeline duration (3s)\n");

    // Queue and sample the actions.
    timeline.queue_actions();
    timeline.sample_queued_actions(
        &world.registry,
        &mut world.subject_world,
    );

    println!(
        "# After sampling: \ncurrent time: {}s,\ntarget time: {}s\n",
        timeline.curr_time(),
        timeline.target_time()
    );
}

impl SubjectSource<Self, Id, Point> for SubjectWorld {
    fn get_source(&self, id: Id) -> Option<&Point> {
        match self.world.get(&id)? {
            Subject::Point(point) => Some(point),
            Subject::Line(_) => None,
        }
    }

    fn apply_source<R>(
        &mut self,
        id: Id,
        f: impl FnOnce(&mut Point) -> R,
    ) -> Option<R> {
        match self.world.get_mut(&id)? {
            Subject::Point(point) => Some(f(point)),
            Subject::Line(_) => None,
        }
    }
}

impl SubjectSource<Self, Id, Line> for SubjectWorld {
    fn get_source(&self, id: Id) -> Option<&Line> {
        match self.world.get(&id)? {
            Subject::Line(line) => Some(line),
            Subject::Point(_) => None,
        }
    }

    fn apply_source<R>(
        &mut self,
        id: Id,
        f: impl FnOnce(&mut Line) -> R,
    ) -> Option<R> {
        match self.world.get_mut(&id)? {
            Subject::Line(line) => Some(f(line)),
            Subject::Point(_) => None,
        }
    }
}

fn linear_f32(a: &f32, b: &f32, t: f32) -> f32 {
    *a + (*b - *a) * t
}

fn linear_point(a: &Point, b: &Point, t: f32) -> Point {
    Point {
        x: linear_f32(&a.x, &b.x, t),
        y: linear_f32(&a.y, &b.y, t),
    }
}
