//! Storage-layer benchmark for motiongfx.
//!
//! This file uses only the public `motiongfx` API, which is identical
//! across the ECS (`main`) and typarena (`nixon/typarena`) branches.
//! That lets the *same* benchmark run on both worktrees so the timing
//! delta is attributable purely to the action-storage backend.
//!
//! Three phases of the animation loop are measured separately:
//!   * `build`  — construct + compile a timeline (insertion into store)
//!   * `bake`   — recompute all segments from subject start values
//!   * `scrub`  — full-playback queue+sample sweep (the per-frame path)
//!
//! Each phase runs twice: a single-type scene (`f32`, one pipeline) and
//! a `mixed_*` scene (four distinct value types, four pipelines) that
//! stresses per-type dispatch and — in bevy ECS — archetype width.
//!
//! Run (see the module docs / your notes for baseline comparison):
//!   cargo bench -p motiongfx

use criterion::{
    BenchmarkId, Criterion, Throughput, criterion_group,
    criterion_main,
};
use std::hint::black_box;

use motiongfx::prelude::*;

/// Subject identifier. Distinct ids give distinct `ActionKey`s.
#[derive(
    Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy,
)]
struct Id(u64);

#[derive(Debug, Default, Clone, Copy)]
#[allow(dead_code)] // `y` rounds out a realistic 2-field subject.
struct Point {
    x: f32,
    y: f32,
}

/// Dense, index-addressable subject world so the benchmark measures
/// motiongfx internals rather than the host container's lookup cost.
struct SubjectWorld {
    points: Vec<Point>,
}

impl SubjectSource<Id, Point> for SubjectWorld {
    fn get_source(&self, id: Id) -> Option<&Point> {
        self.points.get(id.0 as usize)
    }

    fn apply_source<R>(
        &mut self,
        id: Id,
        f: impl FnOnce(&mut Point) -> R,
    ) -> Option<R> {
        self.points.get_mut(id.0 as usize).map(f)
    }
}

fn linear_f32(a: &f32, b: &f32, t: f32) -> f32 {
    *a + (*b - *a) * t
}

fn make_world(n: u64) -> SubjectWorld {
    SubjectWorld {
        points: vec![Point::default(); n as usize],
    }
}

/// Build a timeline with `n` subjects, each animating `Point::x` over a
/// 1s clip. All `n` sequences run concurrently in a single track.
fn build_timeline(
    registry: &mut Registry,
    n: u64,
) -> Timeline<SubjectWorld> {
    let mut builder = registry.create_builder::<SubjectWorld>();

    let fragments = (0..n)
        .map(|i| {
            builder
                .act_builder(Id(i), path!(<Point>::x), |x| x + 72.0)
                .with_interp(linear_f32)
                .play(1.0)
        })
        .collect::<Vec<_>>();

    builder.add_tracks(fragments.ord_all().compile());
    builder.compile()
}

fn bench_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");
    for &n in &[100u64, 1_000, 10_000] {
        group.throughput(Throughput::Elements(n));
        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            &n,
            |b, &n| {
                b.iter(|| {
                    // Fresh registry each iter: measures store
                    // population from scratch.
                    let mut registry = Registry::new();
                    let timeline =
                        build_timeline(&mut registry, black_box(n));
                    black_box((registry, timeline));
                });
            },
        );
    }
    group.finish();
}

fn bench_bake(c: &mut Criterion) {
    let mut group = c.benchmark_group("bake");
    for &n in &[100u64, 1_000, 10_000] {
        group.throughput(Throughput::Elements(n));
        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            &n,
            |b, &n| {
                let mut registry = Registry::new();
                let mut timeline = build_timeline(&mut registry, n);
                let world = make_world(n);
                // bake is idempotent, so re-baking the same timeline
                // measures the segment-recompute cost each iteration.
                b.iter(|| {
                    timeline.bake_actions(
                        black_box(&registry),
                        black_box(&world),
                    );
                });
            },
        );
    }
    group.finish();
}

fn bench_scrub(c: &mut Criterion) {
    const FRAMES: u32 = 60;
    let mut group = c.benchmark_group("scrub");
    for &n in &[100u64, 1_000, 10_000] {
        // Elements = subjects sampled per frame, across all frames.
        group.throughput(Throughput::Elements(n * FRAMES as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            &n,
            |b, &n| {
                let mut registry = Registry::new();
                let mut timeline = build_timeline(&mut registry, n);
                let mut world = make_world(n);
                timeline.bake_actions(&registry, &world);
                let duration = timeline.curr_track().duration();

                // One full playback sweep of FRAMES steps, then back to
                // the start so successive iterations are identical.
                b.iter(|| {
                    for frame in 0..=FRAMES {
                        let t = duration * frame / FRAMES;
                        timeline.set_target_time(black_box(t));
                        timeline.queue_actions();
                        timeline.sample_queued_actions(
                            &registry, &mut world,
                        );
                    }
                    // Rewind to t=0 for the next iteration.
                    timeline.set_target_time(0.0);
                    timeline.queue_actions();
                    timeline
                        .sample_queued_actions(&registry, &mut world);
                    black_box(&world);
                });
            },
        );
    }
    group.finish();
}

// ---------------------------------------------------------------------
// Mixed-type scene.
//
// The benches above use a single value type (`f32`), i.e. one pipeline
// and one column set — the best case for ECS (a single archetype). Real
// scenes animate many value types. `Widget` carries four distinct types
// animated concurrently, producing four pipelines / four heterogeneous
// column sets in typarena and a four-wide archetype in bevy ECS, which
// exercises the per-type dispatch and column-resolution paths.
// ---------------------------------------------------------------------

#[derive(Debug, Default, Clone, Copy)]
struct V2 {
    x: f32,
    y: f32,
}
#[derive(Debug, Default, Clone, Copy)]
struct V3 {
    x: f32,
    y: f32,
    z: f32,
}
#[derive(Debug, Default, Clone, Copy)]
struct V4 {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

/// A subject with four distinct field types, each its own pipeline.
#[derive(Debug, Default, Clone, Copy)]
struct Widget {
    s: f32,
    p: V2,
    r: V3,
    c: V4,
}

struct MixedWorld {
    widgets: Vec<Widget>,
}

impl SubjectSource<Id, Widget> for MixedWorld {
    fn get_source(&self, id: Id) -> Option<&Widget> {
        self.widgets.get(id.0 as usize)
    }

    fn apply_source<R>(
        &mut self,
        id: Id,
        f: impl FnOnce(&mut Widget) -> R,
    ) -> Option<R> {
        self.widgets.get_mut(id.0 as usize).map(f)
    }
}

fn lerp2(a: &V2, b: &V2, t: f32) -> V2 {
    V2 {
        x: linear_f32(&a.x, &b.x, t),
        y: linear_f32(&a.y, &b.y, t),
    }
}
fn lerp3(a: &V3, b: &V3, t: f32) -> V3 {
    V3 {
        x: linear_f32(&a.x, &b.x, t),
        y: linear_f32(&a.y, &b.y, t),
        z: linear_f32(&a.z, &b.z, t),
    }
}
fn lerp4(a: &V4, b: &V4, t: f32) -> V4 {
    V4 {
        x: linear_f32(&a.x, &b.x, t),
        y: linear_f32(&a.y, &b.y, t),
        z: linear_f32(&a.z, &b.z, t),
        w: linear_f32(&a.w, &b.w, t),
    }
}

fn make_mixed_world(n: u64) -> MixedWorld {
    MixedWorld {
        widgets: vec![Widget::default(); n as usize],
    }
}

/// Build a timeline with `n` widgets, each animating all four typed
/// fields over a 1s clip — `4 * n` actions across four pipelines.
fn build_mixed(
    registry: &mut Registry,
    n: u64,
) -> Timeline<MixedWorld> {
    let mut builder = registry.create_builder::<MixedWorld>();

    let fragments = (0..n)
        .flat_map(|i| {
            [
                builder
                    .act_builder(Id(i), path!(<Widget>::s), |v| {
                        v + 1.0
                    })
                    .with_interp(linear_f32)
                    .play(1.0),
                builder
                    .act_builder(
                        Id(i),
                        path!(<Widget>::p),
                        |v: &V2| V2 {
                            x: v.x + 1.0,
                            y: v.y + 1.0,
                        },
                    )
                    .with_interp(lerp2)
                    .play(1.0),
                builder
                    .act_builder(
                        Id(i),
                        path!(<Widget>::r),
                        |v: &V3| V3 {
                            x: v.x + 1.0,
                            y: v.y + 1.0,
                            z: v.z + 1.0,
                        },
                    )
                    .with_interp(lerp3)
                    .play(1.0),
                builder
                    .act_builder(
                        Id(i),
                        path!(<Widget>::c),
                        |v: &V4| V4 {
                            x: v.x + 1.0,
                            y: v.y + 1.0,
                            z: v.z + 1.0,
                            w: v.w + 1.0,
                        },
                    )
                    .with_interp(lerp4)
                    .play(1.0),
            ]
        })
        .collect::<Vec<_>>();

    builder.add_tracks(fragments.ord_all().compile());
    builder.compile()
}

fn bench_mixed_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_build");
    for &n in &[100u64, 1_000, 10_000] {
        group.throughput(Throughput::Elements(n * 4));
        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            &n,
            |b, &n| {
                b.iter(|| {
                    let mut registry = Registry::new();
                    let timeline =
                        build_mixed(&mut registry, black_box(n));
                    black_box((registry, timeline));
                });
            },
        );
    }
    group.finish();
}

fn bench_mixed_bake(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_bake");
    for &n in &[100u64, 1_000, 10_000] {
        group.throughput(Throughput::Elements(n * 4));
        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            &n,
            |b, &n| {
                let mut registry = Registry::new();
                let mut timeline = build_mixed(&mut registry, n);
                let world = make_mixed_world(n);
                b.iter(|| {
                    timeline.bake_actions(
                        black_box(&registry),
                        black_box(&world),
                    );
                });
            },
        );
    }
    group.finish();
}

fn bench_mixed_scrub(c: &mut Criterion) {
    const FRAMES: u32 = 60;
    let mut group = c.benchmark_group("mixed_scrub");
    for &n in &[100u64, 1_000, 10_000] {
        group.throughput(Throughput::Elements(n * 4 * FRAMES as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            &n,
            |b, &n| {
                let mut registry = Registry::new();
                let mut timeline = build_mixed(&mut registry, n);
                let mut world = make_mixed_world(n);
                timeline.bake_actions(&registry, &world);
                let duration = timeline.curr_track().duration();

                b.iter(|| {
                    for frame in 0..=FRAMES {
                        let t = duration * frame / FRAMES;
                        timeline.set_target_time(black_box(t));
                        timeline.queue_actions();
                        timeline.sample_queued_actions(
                            &registry, &mut world,
                        );
                    }
                    timeline.set_target_time(0.0);
                    timeline.queue_actions();
                    timeline
                        .sample_queued_actions(&registry, &mut world);
                    black_box(&world);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_build,
    bench_bake,
    bench_scrub,
    bench_mixed_build,
    bench_mixed_bake,
    bench_mixed_scrub,
);
criterion_main!(benches);
