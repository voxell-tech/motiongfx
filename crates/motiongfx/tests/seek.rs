//! Scrubbing the playhead: a seek can land anywhere, in either
//! direction, and the sampled state must match that instant exactly,
//! not whatever the timeline last happened to visit.

use motiongfx::prelude::*;

mod common;
use common::{
    CUBE, Transform, Vec3, World, lerp_f32, lerp_vec3, sample_at,
    world_starting_at,
};

/// Two chained clips on the same field: jump to the end, then jump
/// straight back into the middle of the first one.
#[test]
fn backward_jump_resamples_an_earlier_clip() {
    let mut registry = Registry::new();
    let mut builder = registry.create_builder::<World>();

    let track = [
        builder
            .act_builder(
                CUBE,
                path!(<Transform>::translation),
                |_| Vec3 {
                    x: 10.0,
                    ..Default::default()
                },
            )
            .with_interp(lerp_vec3)
            .play(s(2)),
        builder
            .act_builder(
                CUBE,
                path!(<Transform>::translation),
                |_| Vec3 {
                    x: 20.0,
                    ..Default::default()
                },
            )
            .with_interp(lerp_vec3)
            .play(s(2)),
    ]
    .ord_chain();

    let mut timeline = builder.compile(track.compile());
    let mut world = world_starting_at(Vec3::default());
    timeline.bake_actions(&registry, &world);

    let end = sample_at(&registry, &mut timeline, &mut world, s(4));
    assert!((end.x - 20.0).abs() < 1e-3, "end.x = {}", end.x);

    // t = 1 sits halfway through the first clip: x should ease
    // 0 -> 10, landing at 5, not stay at the forward pass's 20.
    let back = sample_at(&registry, &mut timeline, &mut world, s(1));
    assert!(
        (back.x - 5.0).abs() < 1e-3,
        "backward jump into an earlier clip did not resample: x = {}",
        back.x
    );
}

/// A gap that sits after an earlier clip must show that clip's end
/// state, even when the seek that lands there never traverses the
/// clip itself - jumping straight back from well past a later clip.
/// Both clips act on `translation::x`, an inner sub-field action, the
/// case reported as "not being sampled" when scrubbing backward.
#[test]
fn backward_jump_into_a_gap_holds_the_preceding_clips_end() {
    let mut registry = Registry::new();
    let mut builder = registry.create_builder::<World>();

    // A: [0,1] -> x=10.  gap [1,2].  B: [2,3] -> x=20.
    let track = [
        builder
            .act_builder(
                CUBE,
                path!(<Transform>::translation::x),
                |_| 10.0,
            )
            .with_interp(lerp_f32)
            .play(s(1)),
        TrackFragment::silent(s(1)),
        builder
            .act_builder(
                CUBE,
                path!(<Transform>::translation::x),
                |_| 20.0,
            )
            .with_interp(lerp_f32)
            .play(s(1)),
    ]
    .ord_chain();

    let mut timeline = builder.compile(track.compile());
    let mut world = world_starting_at(Vec3::default());
    timeline.bake_actions(&registry, &world);

    let end = sample_at(&registry, &mut timeline, &mut world, s(3));
    assert!((end.x - 20.0).abs() < 1e-3, "end.x = {}", end.x);

    // t = 1.5 is the gap right after A ends: x should hold A's end
    // (10), not B's end (20) left over from the forward pass.
    let back =
        sample_at(&registry, &mut timeline, &mut world, ms(1500));
    assert!(
        (back.x - 10.0).abs() < 1e-3,
        "backward jump into the gap after an earlier clip did not \
         resample: x = {}",
        back.x
    );
}
