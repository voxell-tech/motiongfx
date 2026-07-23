//! Demonstrates the [`MotionGfxEditorPlugin`] timeline editor.
//!
//! A row of cubes animates through a single track containing several
//! actions. The editor docks a timeline panel at the bottom of the
//! window: use the play/pause button to control playback and drag on
//! the timeline to scrub. If the track is wider than the window, scroll
//! the panel horizontally to reveal the rest.

use bevy::color::palettes;
use bevy::prelude::*;
use bevy_motiongfx::BevyMotionGfxPlugin;
use bevy_motiongfx::prelude::*;
use motiongfx_editor::MotionGfxEditorPlugin;

const CUBE_COUNT: usize = 6;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            BevyMotionGfxPlugin,
            MotionGfxEditorPlugin,
        ))
        .add_systems(Startup, (setup, spawn_timeline))
        .run();
}

fn spawn_timeline(
    mut commands: Commands,
    mut motiongfx: ResMut<MotionGfxManager>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a row of cubes.
    let mesh = meshes.add(Cuboid::default());
    let mut cubes = Vec::with_capacity(CUBE_COUNT);
    for i in 0..CUBE_COUNT {
        let x = (i as f32) - (CUBE_COUNT as f32 - 1.0) * 0.5;
        let material = materials.add(StandardMaterial::from_color(
            palettes::tailwind::SKY_400,
        ));
        let cube = commands
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(x * 1.5, 0.0, 0.0)
                    .with_scale(Vec3::ZERO),
            ))
            .id();
        cubes.push(cube);
    }

    // Build a single track: each cube grows, then the whole row spins,
    // staggered so the first track has plenty of actions to show.
    let mut b = motiongfx.create_builder();

    let grow = cubes
        .iter()
        .map(|&cube| {
            b.act(cube, path!(<Transform>::scale), |_| Vec3::ONE)
                .with_ease(ease::back::ease_out)
                .play(cs(60))
        })
        .ord_flow(cs(15));

    let spin = cubes
        .iter()
        .map(|&cube| {
            b.act(cube, path!(<Transform>::rotation), |_| {
                Quat::from_rotation_y(std::f32::consts::PI)
            })
            .with_ease(ease::cubic::ease_in_out)
            .play(s(1))
        })
        .ord_flow(cs(10));

    let track = [grow, spin].ord_chain().compile();
    b.add_tracks(track);

    let timeline = b.compile();
    commands.spawn((
        motiongfx.add_timeline(timeline),
        // Start paused; drive playback from the editor.
        RealtimePlayer::new(),
    ));
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera {
            clear_color: Color::srgb(0.02, 0.02, 0.04).into(),
            ..default()
        },
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 14.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
