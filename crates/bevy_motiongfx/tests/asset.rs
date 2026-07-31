//! Guards the shipped `.mgx.ron` assets against format drift: any
//! change to `Scene`'s shape has to update them in the same commit.

#![cfg(feature = "scene")]

use bevy_motiongfx::scene::backend::Backend;
use motiongfx_scene::scene::Scene;

const CUBE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/bevy_examples/assets/scenes/cube.mgx.ron"
));

#[test]
fn cube_scene_deserializes() {
    let scene: Scene<Backend> =
        ron::from_str(CUBE).expect("cube.mgx.ron should parse");

    let subject = &scene.stage.subjects[0];
    assert_eq!(subject.fields.len(), 1);
    assert_eq!(scene.animation.children.len(), 1);

    // Every seeded value resolves in the pool it points at.
    for seed in &subject.fields {
        assert!(
            scene.values.vec3.contains_key(&seed.value),
            "seed {:?} missing from the vec3 column",
            seed.value
        );
    }
}
