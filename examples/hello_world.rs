use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use motiongfx::prelude::*;

fn main() {
    App::new()
        // Bevy plugins
        .add_plugins(DefaultPlugins)
        // Custom plugins
        .add_plugins(motiongfx::MotionGfxPlugin)
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

    // Color.
    let green = Srgba::hex("A9DC76").unwrap().into();

    // Create cubes.
    let mut cubes = Vec::with_capacity(CAPACITY);
    let mesh_handle = meshes.add(Cuboid::default());
    let material_handle = materials.add(StandardMaterial {
        base_color: green,
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
                    Mesh3d(mesh_handle.clone()),
                    transform,
                    MeshMaterial3d(material_handle.clone()),
                ))
                .id();
            cubes.push(id);
        }
    }

    // Generate sequence.
    let mut cube_seqs = Vec::with_capacity(CAPACITY);

    for w in 0..WIDTH {
        for h in 0..HEIGHT {
            let c = w * WIDTH + h;
            let cube = cubes[c];

            let circ_ease = ease::circ::ease_in_out;

            let sequence = [
                commands
                    .entity(cube)
                    .act(field!(<Transform>::scale), |_| {
                        Vec3::splat(0.9)
                    })
                    .with_ease(circ_ease)
                    .play(1.0),
                commands
                    .entity(cube)
                    .act(field!(<Transform>::translation::x), |x| {
                        x + 1.0
                    })
                    .with_ease(circ_ease)
                    .play(1.0),
                commands
                    .entity(cube)
                    .act(field!(<Transform>::rotation), |_| {
                        Quat::from_euler(
                            EulerRot::XYZ,
                            0.0,
                            f32::to_radians(90.0),
                            0.0,
                        )
                    })
                    .with_ease(circ_ease)
                    .play(1.0),
            ]
            .all();

            cube_seqs.push(sequence);
        }
    }

    let sequence = cube_seqs.flow(0.01);

    commands.create_sequence_player([sequence]);
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera {
            hdr: true,
            ..default()
        },
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 15.0),
        Tonemapping::AcesFitted,
        bevy::core_pipeline::bloom::Bloom::default(),
    ));

    // Directional light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn timeline_movement(
    mut q_timelines: Query<&mut SequencePlayer>,
    keys: Res<ButtonInput<KeyCode>>,
    mut is_playing: Local<bool>,
) {
    for mut player in q_timelines.iter_mut() {
        if *is_playing {
            player.play();
        } else {
            player.pause();
        }

        if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
            player.set_time_scale(1.0).play();
        }

        if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
            player.set_time_scale(-1.0).play();
        }

        if keys.just_pressed(KeyCode::Space) {
            *is_playing = true;
            if keys.pressed(KeyCode::ShiftLeft) {
                player.set_time_scale(-1.0);
            } else {
                player.set_time_scale(1.0);
            }
        }

        if keys.just_pressed(KeyCode::Escape) {
            *is_playing = false;
        }
    }
}
