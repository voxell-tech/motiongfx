use bevy::{core_pipeline::tonemapping::Tonemapping, pbr::NotShadowCaster, prelude::*};
use bevy_motiongfx::{prelude::*, MotionGfxPlugin};

fn main() {
    App::new()
        // Bevy plugins
        .add_plugins(DefaultPlugins)
        // Custom plugins
        .add_plugins(MotionGfxPlugin)
        .add_systems(Startup, (setup, hello_world))
        .add_systems(Update, timeline_movement)
        .run();
}

fn hello_world(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    const WIDTH: usize = 10;
    const HEIGHT: usize = 10;

    const CAPACITY: usize = WIDTH * HEIGHT;

    // Color palette
    let palette = ColorPalette::default();

    // Create cubes
    let mut cubes = Vec::with_capacity(CAPACITY);
    let mesh_handle = meshes.add(Cuboid::default());
    let material_handle = materials.add(StandardMaterial {
        base_color: palette.get(ColorKey::Green),
        ..default()
    });

    for w in 0..WIDTH {
        for h in 0..HEIGHT {
            let transform = Transform::from_translation(Vec3::new(
                (w as f32) - (WIDTH as f32) * 0.5 - 1.0,
                (h as f32) - (HEIGHT as f32) * 0.5,
                0.0,
            ))
            .with_scale(Vec3::ZERO);
            let id = commands
                .spawn((
                    NotShadowCaster,
                    PbrBundle {
                        transform,
                        mesh: mesh_handle.clone(),
                        material: material_handle.clone(),
                        ..default()
                    },
                ))
                .id();
            cubes.push((id, transform));
        }
    }

    // Generate sequence
    let mut cube_seqs = Vec::with_capacity(CAPACITY);

    for w in 0..WIDTH {
        for h in 0..HEIGHT {
            let c = w * WIDTH + h;
            let cube = &mut cubes[c];

            let circ_ease = ease::circ::ease_in_out;

            let sequence = commands
                .add_motion(
                    cube.transform()
                        .to_scale(Vec3::splat(0.9))
                        .with_ease(circ_ease)
                        .animate(1.0),
                )
                .add_motion({
                    let x = cube.transform().transform.translation.x;
                    cube.transform()
                        .to_translation_x(x + 1.0)
                        .with_ease(circ_ease)
                        .animate(1.0)
                })
                .add_motion(
                    cube.transform()
                        .to_rotation(Quat::from_euler(
                            EulerRot::XYZ,
                            0.0,
                            f32::to_radians(90.0),
                            0.0,
                        ))
                        .with_ease(circ_ease)
                        .animate(1.0),
                )
                .all();

            cube_seqs.push(sequence);
        }
    }

    let sequence = cube_seqs.flow(0.01);

    commands.spawn(SequencePlayerBundle {
        sequence,
        ..default()
    });
}

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn(Camera3dBundle {
        camera: Camera {
            hdr: true,
            ..default()
        },
        transform: Transform::from_xyz(-0.5, -0.5, 15.0),
        tonemapping: Tonemapping::AcesFitted,
        ..default()
    });

    // Directional light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(3.0, 10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn timeline_movement(
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
