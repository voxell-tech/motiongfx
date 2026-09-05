//! Guards the shipped `.mgx.ron` assets against format drift: any
//! change to `Scene`'s shape has to update them in the same commit.

#![cfg(feature = "scene")]

use bevy_motiongfx::scene::backend::Backend;
use motiongfx_scene::block::Node;
use motiongfx_scene::scene::Scene;

const CUBE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/bevy_examples/assets/scenes/cube.mgx.ron"
));

#[test]
fn cube_scene_deserializes() {
    let scene: Scene<Backend> =
        ron::from_str(CUBE).expect("cube.mgx.ron should parse");

    // Which column a value belongs in follows from its field's type,
    // which only the registry knows, so a dangling id is what's
    // checkable here.
    let resolves = |id| {
        scene.values.f32.contains_key(&id)
            || scene.values.vec3.contains_key(&id)
            || scene.values.quat.contains_key(&id)
    };

    assert_eq!(scene.stage.subjects.len(), 1);
    assert_eq!(scene.stage.subjects[0].fields.len(), 1);
    assert_eq!(scene.animation.children.len(), 1);

    for subject in &scene.stage.subjects {
        for seed in &subject.fields {
            assert!(
                resolves(seed.value),
                "seed {:?} on subject {:?} missing from the value pool",
                seed.value,
                subject.id
            );
        }
    }

    let mut blocks = vec![&scene.animation];
    while let Some(block) = blocks.pop() {
        for node in &block.children {
            match node {
                Node::Block { block, .. } => blocks.push(block),
                Node::Action { action, .. } => {
                    assert!(
                        resolves(action.value),
                        "action value {:?} on subject {:?} missing from the value pool",
                        action.value,
                        action.subject
                    );
                }
                Node::Draft { .. } => {}
            }
        }
    }
}
