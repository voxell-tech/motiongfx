use bevy::color::palettes;
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
    // Spawn cube.
    let cube = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: palettes::tailwind::LIME_200.into(),
                ..default()
            })),
            Transform::from_xyz(-5.0, 0.0, 0.0),
        ))
        .id();

    // Generate sequence.
    let sequence = commands
        .entity(cube)
        .act(field!(<Transform>::translation::x), |x| x + 10.0)
        // A custom 10 step easing.
        .with_ease(|t| ((t * 10.0) as u32) as f32 / 10.0)
        .play(1.0);

    commands
        .create_timeline(sequence)
        .insert(TimelinePlayback::Forward);
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 15.0),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
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
