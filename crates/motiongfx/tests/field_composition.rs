//! Actions on overlapping field paths of one subject: non-conflicting
//! ones compose through baking, conflicting ones drop the earlier at
//! compile.

use std::collections::HashMap;
use std::time::Duration;

use motiongfx::prelude::*;

const CUBE: u32 = 0;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct Transform {
    translation: Vec3,
}

#[derive(Default)]
struct World {
    subjects: HashMap<u32, Transform>,
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

fn lerp_f32(from: &f32, to: &f32, t: f32) -> f32 {
    from + (to - from) * t
}

fn lerp_vec3(from: &Vec3, to: &Vec3, t: f32) -> Vec3 {
    Vec3 {
        x: lerp_f32(&from.x, &to.x, t),
        y: lerp_f32(&from.y, &to.y, t),
        z: lerp_f32(&from.z, &to.z, t),
    }
}

fn world_starting_at(translation: Vec3) -> World {
    World {
        subjects: HashMap::from([(CUBE, Transform { translation })]),
    }
}

fn sample_at(
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

/// `translation::x` then a chained (non-overlapping) whole
/// `translation`. The second action bakes its start from the composed
/// `x`, so `x` eases from 6, not from the initial 0.
#[test]
fn sub_field_then_whole_field_composes() {
    let mut registry = Registry::new();
    let mut builder = registry.create_builder::<World>();

    let track = [
        builder
            .act_builder(
                CUBE,
                path!(<Transform>::translation::x),
                |_| 6.0,
            )
            .with_interp(lerp_f32)
            .play(s(2)),
        builder
            .act_builder(
                CUBE,
                path!(<Transform>::translation),
                |_| Vec3 {
                    x: 3.0,
                    y: 1.0,
                    z: 2.0,
                },
            )
            .with_interp(lerp_vec3)
            .play(s(2)),
    ]
    .ord_chain();

    let mut timeline = builder.compile(track.compile());
    let mut world = world_starting_at(Vec3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    });
    timeline.bake_actions(&registry, &world);

    // Step past `translation::x` so only the whole-`translation` clip
    // is live, then land halfway through it.
    sample_at(&registry, &mut timeline, &mut world, cs(210));
    let translation =
        sample_at(&registry, &mut timeline, &mut world, s(3));

    assert!(
        (translation.x - 4.5).abs() < 1e-3,
        "x should ease 6 -> 3, got {}",
        translation.x,
    );
}

/// Whole `translation` then a chained `translation::x`. `translation::x`
/// bakes its start from where `translation` left `x`.
#[test]
fn whole_field_then_sub_field_composes() {
    let mut registry = Registry::new();
    let mut builder = registry.create_builder::<World>();

    let track = [
        builder
            .act_builder(
                CUBE,
                path!(<Transform>::translation),
                |_| Vec3 {
                    x: 5.0,
                    y: 5.0,
                    z: 5.0,
                },
            )
            .with_interp(lerp_vec3)
            .play(s(2)),
        builder
            .act_builder(
                CUBE,
                path!(<Transform>::translation::x),
                |_| 9.0,
            )
            .with_interp(lerp_f32)
            .play(s(2)),
    ]
    .ord_chain();

    let mut timeline = builder.compile(track.compile());
    let mut world = world_starting_at(Vec3::default());
    timeline.bake_actions(&registry, &world);

    sample_at(&registry, &mut timeline, &mut world, cs(210));
    let translation =
        sample_at(&registry, &mut timeline, &mut world, s(3));

    assert!(
        (translation.x - 7.0).abs() < 1e-3,
        "x should ease 5 -> 9, got {}",
        translation.x,
    );
    assert!(
        (translation.y - 5.0).abs() < 1e-3
            && (translation.z - 5.0).abs() < 1e-3,
        "y and z hold where `translation` left them, got {translation:?}",
    );
}

/// A seek that lands inside a later `translation::x` clip while the
/// earlier whole-`translation` clip resolves to its end in the same
/// pass. The `x` slide is newer and wins, whichever pipeline samples
/// first.
#[test]
fn boundary_crossing_seek_applies_child_alias_last() {
    let mut registry = Registry::new();
    let mut builder = registry.create_builder::<World>();

    let track = [
        builder
            .act_builder(
                CUBE,
                path!(<Transform>::translation),
                |_| Vec3 {
                    x: 2.0,
                    y: 5.0,
                    z: 5.0,
                },
            )
            .with_interp(lerp_vec3)
            .play(s(2)),
        builder
            .act_builder(
                CUBE,
                path!(<Transform>::translation::x),
                |_| 9.0,
            )
            .with_interp(lerp_f32)
            .play(s(2)),
    ]
    .ord_chain();

    let mut timeline = builder.compile(track.compile());
    let mut world = world_starting_at(Vec3::default());
    timeline.bake_actions(&registry, &world);

    // From t = 0 straight to t = 3: past the whole-`translation` clip
    // [0, 2] (-> its end) and halfway through `translation::x` [2, 4].
    let translation =
        sample_at(&registry, &mut timeline, &mut world, s(3));

    assert!(
        (translation.x - 5.5).abs() < 1e-3,
        "x eases 2 -> 9 and is not clobbered by `translation`'s end, got {}",
        translation.x,
    );
    assert!(
        (translation.y - 5.0).abs() < 1e-3
            && (translation.z - 5.0).abs() < 1e-3,
        "y and z hold where `translation` ended, got {translation:?}",
    );
}

/// `translation::x` and `translation::y` fully overlap in time but do
/// not alias, so both run.
#[test]
fn sibling_sub_fields_do_not_conflict() {
    let mut registry = Registry::new();
    let mut builder = registry.create_builder::<World>();

    let track = [
        builder
            .act_builder(
                CUBE,
                path!(<Transform>::translation::x),
                |_| 6.0,
            )
            .with_interp(lerp_f32)
            .play(s(4)),
        builder
            .act_builder(
                CUBE,
                path!(<Transform>::translation::y),
                |_| 8.0,
            )
            .with_interp(lerp_f32)
            .play(s(4)),
    ]
    .ord_all();

    let mut timeline = builder.compile(track.compile());
    let mut world = world_starting_at(Vec3::default());
    timeline.bake_actions(&registry, &world);

    let translation =
        sample_at(&registry, &mut timeline, &mut world, s(2));

    assert!(
        (translation.x - 3.0).abs() < 1e-3,
        "x eases 0 -> 6: {}",
        translation.x,
    );
    assert!(
        (translation.y - 4.0).abs() < 1e-3,
        "y eases 0 -> 8: {}",
        translation.y,
    );
}

/// Whole `translation` then an overlapping `translation::x`. The
/// `translation` clip is dropped at compile, so only the `x` slide
/// runs and `y`/`z` never move.
#[test]
fn overlapping_whole_and_sub_field_drops_the_earlier() {
    let mut registry = Registry::new();
    let mut builder = registry.create_builder::<World>();

    let track = [
        builder
            .act_builder(
                CUBE,
                path!(<Transform>::translation),
                |_| Vec3 {
                    x: 5.0,
                    y: 5.0,
                    z: 5.0,
                },
            )
            .with_interp(lerp_vec3)
            .play(s(4)),
        builder
            .act_builder(
                CUBE,
                path!(<Transform>::translation::x),
                |_| 9.0,
            )
            .with_interp(lerp_f32)
            .play(s(2)),
    ]
    .ord_flow(s(1));

    let mut timeline = builder.compile(track.compile());
    let mut world = world_starting_at(Vec3::default());
    timeline.bake_actions(&registry, &world);

    // The surviving `translation::x` clip is [1, 3]; halfway is t = 2.
    let translation =
        sample_at(&registry, &mut timeline, &mut world, s(2));

    assert!(
        (translation.x - 4.5).abs() < 1e-3,
        "x eases 0 -> 9: {}",
        translation.x,
    );
    assert!(
        translation.y == 0.0 && translation.z == 0.0,
        "`translation`'s y/z motion was dropped, got {translation:?}",
    );
}
