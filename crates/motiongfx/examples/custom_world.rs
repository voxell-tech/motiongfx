use std::collections::HashMap;

use motiongfx::prelude::*;

struct World {
    accessor_registry: FieldAccessorRegistry,
    pipeline_registry: PipelineRegistry<SubjectWorld>,
    subject_world: SubjectWorld,
}

impl World {
    pub fn new() -> Self {
        Self {
            accessor_registry: FieldAccessorRegistry::new(),
            pipeline_registry: PipelineRegistry::new(),
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

    // Register the accessors.
    register_accessors(&mut world.accessor_registry);
    // Regsitre the pipelines.
    register_pipelines(&mut world.pipeline_registry);

    // Spawn in some subjects.
    world
        .subject_world
        .world
        .insert(Id(0), Subject::Point(Point::default()));
    world
        .subject_world
        .world
        .insert(Id(1), Subject::Line(Line::default()));

    let mut builder = TimelineBuilder::new();

    // Create the track.
    let track = [
        builder
            .act(Id(0), field!(<Point>::x), |x| x + 72.0)
            .with_interp(linear_f32)
            .play(1.0),
        [
            builder
                .act(Id(1), field!(<Line>::p0::y), |y| y + 42.0)
                .with_interp(linear_f32)
                .play(2.0),
            builder
                .act(Id(1), field!(<Line>::p1), |_| Point {
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
    timeline.bake_actions(
        &world.accessor_registry,
        &world.pipeline_registry,
        &world.subject_world,
    );

    // Change the target time.
    timeline.set_target_time(1.5);

    // Check the values before sampling:
    println!("Before: {:?}", world.subject_world);

    // Queue and sample the actions.
    timeline.queue_actions();
    timeline.sample_queued_actions(
        &world.accessor_registry,
        &world.pipeline_registry,
        &mut world.subject_world,
    );

    // Check the values after sampling:
    println!("After:  {:?}", world.subject_world);

    timeline.set_target_time(7.0);
    println!();

    println!(
        "current time: {},\ntarget time: {}",
        timeline.curr_time(),
        timeline.target_time()
    );

    // Queue and sample the actions.
    timeline.queue_actions();
    timeline.sample_queued_actions(
        &world.accessor_registry,
        &world.pipeline_registry,
        &mut world.subject_world,
    );
    println!();

    println!(
        "current time: {},\ntarget time: {}",
        timeline.curr_time(),
        timeline.target_time()
    );

    println!("Timeline is complete: {}", timeline.is_complete());
}

fn register_pipelines(
    pipeline_registry: &mut PipelineRegistry<SubjectWorld>,
) {
    pipeline_registry.register_unchecked(
        PipelineKey::new::<Id, Point, f32>(),
        Pipeline::new(
            |world, ctx| {
                ctx.bake::<Id, Point, f32>(|id| {
                    let subject = world.world.get(&id)?;
                    match subject {
                        Subject::Point(point) => Some(point),
                        Subject::Line(_) => None,
                    }
                });
            },
            |world, ctx| {
                ctx.sample::<Id, Point, f32>(
                    |id, target, accessor| {
                        let Some(subject) = world.world.get_mut(&id)
                        else {
                            return;
                        };

                        if let Subject::Point(point) = subject {
                            *accessor.get_mut(point) = target;
                        }
                    },
                );
            },
        ),
    );

    pipeline_registry.register_unchecked(
        PipelineKey::new::<Id, Line, Point>(),
        Pipeline::new(
            |world, ctx| {
                ctx.bake::<Id, Line, Point>(|id| {
                    let subject = world.world.get(&id)?;
                    match subject {
                        Subject::Line(line) => Some(line),
                        Subject::Point(_) => None,
                    }
                });
            },
            |world, ctx| {
                ctx.sample::<Id, Line, Point>(
                    |id, target, accessor| {
                        let Some(subject) = world.world.get_mut(&id)
                        else {
                            return;
                        };

                        if let Subject::Line(line) = subject {
                            *accessor.get_mut(line) = target;
                        }
                    },
                );
            },
        ),
    );

    // TODO: This looks almost exactly the same as above, could
    // pipeline be simplified enough to ignore the target field?
    pipeline_registry.register_unchecked(
        PipelineKey::new::<Id, Line, f32>(),
        Pipeline::new(
            |world, ctx| {
                ctx.bake::<Id, Line, f32>(|id| {
                    let subject = world.world.get(&id)?;
                    match subject {
                        Subject::Line(line) => Some(line),
                        Subject::Point(_) => None,
                    }
                });
            },
            |world, ctx| {
                ctx.sample::<Id, Line, f32>(
                    |id, target, accessor| {
                        let Some(subject) = world.world.get_mut(&id)
                        else {
                            return;
                        };

                        if let Subject::Line(line) = subject {
                            *accessor.get_mut(line) = target;
                        }
                    },
                );
            },
        ),
    );
}

fn register_accessors(accessor_registry: &mut FieldAccessorRegistry) {
    // In real use cases, a macro should be used!
    // Refer to `bevy_motiongfx` for now...

    // Point -> f32.
    accessor_registry
        .register_typed(field!(<Point>::x), accessor!(<Point>::x));
    accessor_registry
        .register_typed(field!(<Point>::y), accessor!(<Point>::y));
    // Line -> Point.
    accessor_registry
        .register_typed(field!(<Line>::p0), accessor!(<Line>::p0));
    accessor_registry
        .register_typed(field!(<Line>::p1), accessor!(<Line>::p1));
    // Line -> Point -> f32.
    accessor_registry.register_typed(
        field!(<Line>::p0::x),
        accessor!(<Line>::p0::x),
    );
    accessor_registry.register_typed(
        field!(<Line>::p0::y),
        accessor!(<Line>::p0::y),
    );
    accessor_registry.register_typed(
        field!(<Line>::p1::x),
        accessor!(<Line>::p1::x),
    );
    accessor_registry.register_typed(
        field!(<Line>::p1::y),
        accessor!(<Line>::p1::y),
    );
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
