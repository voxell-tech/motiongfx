use bevy::{
    core_pipeline::{
        bloom::BloomSettings,
        experimental::taa::{TemporalAntiAliasBundle, TemporalAntiAliasPlugin},
    },
    pbr::{NotShadowCaster, ScreenSpaceAmbientOcclusionBundle},
    prelude::*,
};
use bevy_motiongfx::prelude::*;

fn main() {
    App::new()
        // Bevy plugins
        .add_plugins((DefaultPlugins, TemporalAntiAliasPlugin))
        .insert_resource(Msaa::Off)
        // Custom plugins
        .add_plugins((MotionGfx, MotionGfxBevy))
        .add_systems(Startup, (setup_system, hello_world_system))
        .add_systems(Update, timeline_movement_system)
        .run();
}

fn hello_world_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const WIDTH: usize = 10;
    const HEIGHT: usize = 10;

    const CAPACITY: usize = WIDTH * HEIGHT;

    // Color palette
    let palette = ColorPalette::default();

    let mut cubes = Vec::with_capacity(CAPACITY);
    // Motion
    let mut transform_motions = Vec::with_capacity(CAPACITY);

    // Create cube objects (Entity)
    let material = StandardMaterial {
        base_color: *palette.get_or_default(&ColorKey::Green),
        ..default()
    };

    for w in 0..WIDTH {
        for h in 0..HEIGHT {
            let transform = Transform::from_translation(Vec3::new(
                (w as f32) - (WIDTH as f32) * 0.5 - 1.0,
                (h as f32) - (HEIGHT as f32) * 0.5,
                0.0,
            ))
            .with_scale(Vec3::ZERO);

            let cube = commands
                .spawn(PbrBundle {
                    transform,
                    mesh: meshes.add(Cuboid::default()),
                    material: materials.add(material.clone()),
                    ..default()
                })
                .insert(NotShadowCaster)
                .id();

            transform_motions.push(TransformMotion::new(cube, transform));

            cubes.push(cube);
        }
    }

    // Generate cube animations
    let mut cube_seqs = Vec::with_capacity(CAPACITY);

    for w in 0..WIDTH {
        for h in 0..HEIGHT {
            let c = w * WIDTH + h;

            cube_seqs.push(
                all(&[
                    commands.play(transform_motions[c].translate_add(Vec3::X), 1.0),
                    commands.play(transform_motions[c].scale_to(Vec3::splat(0.9)), 1.0),
                    commands.play(
                        transform_motions[c].rotate_to(Quat::from_euler(
                            EulerRot::XYZ,
                            0.0,
                            f32::to_radians(90.0),
                            0.0,
                        )),
                        1.0,
                    ),
                ])
                .with_ease(ease::circ::ease_in_out),
            );
        }
    }

    let sequence = flow(0.01, &cube_seqs);

    commands.spawn(SequencePlayerBundle {
        sequence,
        ..default()
    });
}

fn setup_system(mut commands: Commands) {
    // Camera
    commands
        .spawn(Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(-0.5, -0.5, 15.0),
            tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::AcesFitted,
            ..default()
        })
        .insert(BloomSettings::default())
        .insert(ScreenSpaceAmbientOcclusionBundle::default())
        .insert(TemporalAntiAliasBundle::default());

    // Directional light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(3.0, 10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn timeline_movement_system(
    mut q_timelines: Query<(&mut SequencePlayer, &mut SequenceController)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    for (mut sequence_player, mut sequence_time) in q_timelines.iter_mut() {
        if keys.pressed(KeyCode::KeyD) {
            sequence_time.target_time += time.delta_seconds();
        }

        if keys.pressed(KeyCode::KeyA) {
            sequence_time.target_time -= time.delta_seconds();
        }

        if keys.just_pressed(KeyCode::Space) {
            if keys.pressed(KeyCode::ShiftLeft) {
                sequence_player.time_scale = -1.0;
            } else {
                sequence_player.time_scale = 1.0;
            }
        }

        if keys.just_pressed(KeyCode::Escape) {
            sequence_player.time_scale = 0.0;
        }
    }
}
