use bevy::color::palettes;
use bevy::core_pipeline::bloom::Bloom;
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use motiongfx::prelude::*;

fn main() {
    App::new()
        // Bevy plugins
        .add_plugins(DefaultPlugins)
        // Custom plugins
        .add_plugins(motiongfx::MotionGfxPlugin)
        .add_systems(Startup, (setup, spawn_timeline))
        .add_systems(Update, timeline_movement)
        .run();
}

fn spawn_timeline(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let easings = [
        ease::linear,
        ease::sine::ease_in_out,
        ease::quad::ease_in_out,
        ease::cubic::ease_in_out,
        ease::quart::ease_in_out,
        ease::quint::ease_in_out,
        ease::expo::ease_in_out,
        ease::circ::ease_in_out,
        ease::back::ease_in_out,
        ease::elastic::ease_in_out,
    ];

    let capacity = easings.len();

    // Colors.
    let blue = LinearRgba::from(palettes::tailwind::CYAN_300) * 100.0;
    let red = LinearRgba::from(palettes::tailwind::ROSE_400) * 100.0;

    // Spawn spheres.
    let mut spheres = Vec::with_capacity(capacity);
    let mesh_handle = meshes.add(Sphere::default());
    let material = StandardMaterial {
        base_color: Color::WHITE,
        emissive: blue,
        ..default()
    };

    for i in 0..capacity {
        let sphere = commands
            .spawn((
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(materials.add(material.clone())),
                Transform::from_translation(Vec3::new(
                    -5.0,
                    (i as f32) - (capacity as f32) * 0.5,
                    0.0,
                ))
                .with_scale(Vec3::ONE),
                NotShadowCaster,
            ))
            .id();

        spheres.push(sphere);
    }

    // Generate sequence.
    let sequence = spheres
        .iter()
        .zip(easings)
        .map(|(&sphere, ease_fn)| {
            [
                commands
                    .entity(sphere)
                    .act(field!(<Transform>::translation::x), |x| {
                        x + 10.0
                    })
                    .with_ease(ease_fn)
                    .play(1.0),
                commands
                    .entity(sphere)
                    .act(
                        field!(<StandardMaterial>::emissive),
                        move |_| red,
                    )
                    .with_ease(ease_fn)
                    .play(1.0),
            ]
            .all()
        })
        .collect::<Vec<_>>()
        .chain();

    commands
        .create_timeline(sequence)
        .insert(TimelinePlayback::Forward);
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera {
            hdr: true,
            clear_color: Color::BLACK.into(),
            ..default()
        },
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 15.0),
        Bloom::default(),
    ));
}

fn timeline_movement(
    mut q_timelines: Query<(&Timeline, &mut TimelinePlayback)>,
    mut q_sequences: Query<&mut SequenceController>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) -> Result {
    for (timeline, mut playback) in q_timelines.iter_mut() {
        let mut controller = timeline
            .curr_sequence_id()
            .and_then(|e| q_sequences.get_mut(e).ok())
            .ok_or("Can't get sequence controller!")?;

        if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
            controller.target_time += time.delta_secs();
        }

        if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
            controller.target_time -= time.delta_secs();
        }

        if keys.just_pressed(KeyCode::Space) {
            if keys.pressed(KeyCode::ShiftLeft) {
                playback.backward();
            } else {
                playback.forward();
            }
        }

        if keys.just_pressed(KeyCode::Escape) {
            playback.pause();
        }
    }

    Ok(())
}
