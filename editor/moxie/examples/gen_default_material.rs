//! Generates a default material file at `editor/assets/materials/default.mat`.
//!
//! Run with `cargo run -p moxie --example gen_default_material`.

use std::path::Path;

use bevy::ecs::reflect::AppTypeRegistry;
use bevy::pbr::StandardMaterial;
use bevy::prelude::*;
use moxie::std_material_asset::serialize_to_ron;

fn main() {
    let mut world = World::new();
    world.init_resource::<AppTypeRegistry>();
    {
        let registry = world.resource_mut::<AppTypeRegistry>();
        registry.write().register::<StandardMaterial>();
    }
    let registry = world.resource::<AppTypeRegistry>().read();
    let ron =
        serialize_to_ron(&StandardMaterial::default(), &registry)
            .expect("should serialize default material");

    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../assets/materials/default.mat");
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    std::fs::write(&path, ron).expect("should write default.mat");
}
