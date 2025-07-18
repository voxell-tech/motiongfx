use std::f32::consts::FRAC_PI_2;

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
        .act(field!(<Transform>::translation), |x| {
            x + Vec3::ZERO.with_x(10.0).with_z(1.0)
        })
        .with_ease(ease::cubic::ease_in_out)
        .with_interp(|start, end, t| arc_lerp_3d(*start, *end, t))
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
        // Top down view.
        Transform::from_xyz(0.0, 18.0, 0.0)
            .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
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

// TODO: Optimize this.
pub fn arc_lerp_3d(start: Vec3, end: Vec3, t: f32) -> Vec3 {
    let center = (start + end) * 0.5;

    let start_dir = Dir3::new(start - center);
    let end_dir = Dir3::new(end - center);

    let (Ok(start_dir), Ok(end_dir)) = (start_dir, end_dir) else {
        // Revert to linear interpolation.
        return start.lerp(end, t);
    };

    let target_dir = start_dir.slerp(end_dir, t);

    center + target_dir.as_vec3() * (center - start).length()
}
