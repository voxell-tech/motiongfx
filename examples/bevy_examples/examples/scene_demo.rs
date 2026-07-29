//! Loads `assets/scenes/cube.mgx.ron` through the real
//! `SceneAssetLoader` (not built by hand in Rust), then spawns its
//! subject, compiles it, and plays it. Proves the whole scene
//! pipeline end to end: `.mgx.ron` file -> `AssetServer` -> registry
//! -> compile -> spawned `Transform` -> played `Timeline`.

use bevy::prelude::*;
use bevy_motiongfx::BevyMotionGfxPlugin;
use bevy_motiongfx::prelude::*;
use bevy_motiongfx::scene::asset::MotionGfxScene;
use bevy_motiongfx::scene::backend::default_scene_registry;
use bevy_motiongfx::scene::spawn_scene;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BevyMotionGfxPlugin))
        .add_systems(Startup, (setup, load_scene))
        .add_systems(Update, build_scene_once_loaded)
        .run();
}

/// Spawns the camera and the directional light.
fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 15.0),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[derive(Resource)]
struct PendingScene(Handle<MotionGfxScene>);

fn load_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let handle = asset_server.load("scenes/cube.mgx.ron");
    commands.insert_resource(PendingScene(handle));
}

/// Runs every frame until the asset finishes loading, then spawns its
/// subject, compiles it, and plays it - exactly once.
fn build_scene_once_loaded(
    mut commands: Commands,
    pending: Option<Res<PendingScene>>,
    scenes: Res<Assets<MotionGfxScene>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut motiongfx: ResMut<MotionGfxManager>,
) {
    let Some(pending) = pending else {
        return;
    };
    let Some(scene) = scenes.get(&pending.0) else {
        return;
    };
    // Materializes the scene's one subject into a real entity, then
    // attaches the visuals (the scene format itself doesn't know
    // about meshes/materials - only `Transform`).
    for (_, entity) in spawn_scene(&mut commands, scene) {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(
                materials
                    .add(StandardMaterial::from_color(Srgba::BLUE)),
            ),
            Transform::from_xyz(-3.0, 0.0, 0.0),
        ));
    }

    let registry = default_scene_registry();
    let timeline_id = scene
        .compile(&registry, &mut motiongfx)
        .expect("scene should compile");

    commands.spawn((
        timeline_id,
        RealtimePlayer::new().with_playing(true),
    ));

    commands.remove_resource::<PendingScene>();
    info!("scene loaded, compiled, and playing");
}
