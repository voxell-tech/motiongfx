//! Loads `assets/scenes/cube.scn.ron` (entity composition, via
//! `bevy_world_serialization`) and `assets/scenes/cube.mgx.ron`
//! (animation, via `SceneAssetLoader`), waits for both, then compiles
//! and plays it. Proves the whole pipeline end to end: composition
//! file + animation file -> spawned entities -> compiled `Timeline`.

use bevy::prelude::*;
use bevy::world_serialization::{
    DynamicWorld, DynamicWorldRoot, WorldInstanceReady,
};
use bevy_motiongfx::BevyMotionGfxPlugin;
use bevy_motiongfx::prelude::*;
use bevy_motiongfx::scene::asset::MotionGfxScene;
use bevy_motiongfx::scene::backend::default_scene_registry;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BevyMotionGfxPlugin))
        .register_type::<Cube>()
        .init_resource::<CompositionReady>()
        .add_systems(Startup, (setup, load_scene))
        .add_systems(Update, build_scene_once_loaded)
        .add_observer(on_composition_ready)
        .add_observer(on_add_cube)
        .run();
}

/// Marks an entity as a plain cube - the composition file attaches
/// this alongside `Transform`/`EntityUid` for whatever should render
/// as one, and [`on_add_cube`] gives it a mesh/material the moment it
/// shows up.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Cube;

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

#[derive(Resource, Default)]
struct CompositionReady(bool);

fn load_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let composition: Handle<DynamicWorld> =
        asset_server.load("scenes/cube.scn.ron");
    commands.spawn(DynamicWorldRoot(composition));

    let handle = asset_server.load("scenes/cube.mgx.ron");
    commands.insert_resource(PendingScene(handle));
}

fn on_composition_ready(
    _trigger: On<WorldInstanceReady>,
    mut ready: ResMut<CompositionReady>,
) {
    ready.0 = true;
}

/// Gives a mesh/material to whatever entity just got a [`Cube`] -
/// composition files carry the marker, not the asset handles
/// themselves, since meshes/materials aren't part of this format yet.
fn on_add_cube(
    trigger: On<Add, Cube>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.entity(trigger.entity).insert((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(
            materials.add(StandardMaterial::from_color(Srgba::BLUE)),
        ),
    ));
}

/// Runs every frame until both the composition and the animation are
/// ready, then compiles and plays it - exactly once.
fn build_scene_once_loaded(
    mut commands: Commands,
    pending: Option<Res<PendingScene>>,
    scenes: Res<Assets<MotionGfxScene>>,
    composition_ready: Res<CompositionReady>,
    mut motiongfx: ResMut<MotionGfxManager>,
) {
    if !composition_ready.0 {
        return;
    }
    let Some(pending) = pending else {
        return;
    };
    let Some(scene) = scenes.get(&pending.0) else {
        return;
    };

    let registry = default_scene_registry();
    let timeline_id = scene
        .compile(&registry, &mut motiongfx)
        .expect("scene should compile");

    // Needs `&mut World`, so it defers to the next command flush -
    // still well before `MotionGfxSystems::Sample` bakes in
    // `PostUpdate`.
    let handle = pending.0.clone();
    commands.queue(move |world: &mut World| {
        let registry = default_scene_registry();
        world.resource_scope::<Assets<MotionGfxScene>, _>(
            |world, scenes| {
                scenes
                    .get(&handle)
                    .expect("scene should still be loaded")
                    .stage(&registry, world)
                    .expect("stage should apply");
            },
        );
    });

    commands.spawn((
        timeline_id,
        RealtimePlayer::new().with_playing(true),
    ));

    commands.remove_resource::<PendingScene>();
    info!("scene loaded, compiled, and playing");
}
