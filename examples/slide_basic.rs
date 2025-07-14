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
        .add_systems(Update, slide_movement)
        .run();
}

fn spawn_timeline(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const X_OFFSET: f32 = 2.0;

    // Cube.
    let transform = Transform::default().with_scale(Vec3::splat(0.0));
    let material = StandardMaterial {
        base_color: palettes::tailwind::LIME_200.into(),
        ..default()
    };
    let material_handle = materials.add(material.clone());
    let cube = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            transform,
            MeshMaterial3d(material_handle.clone()),
        ))
        .id();

    // Sphere.
    let transform = Transform::default()
        .with_translation(Vec3::X * X_OFFSET)
        .with_scale(Vec3::splat(0.0));
    let material = StandardMaterial {
        base_color: palettes::tailwind::CYAN_300.into(),
        ..default()
    };
    let material_handle = materials.add(material.clone());
    let sphere = commands
        .spawn((
            Mesh3d(meshes.add(Sphere::default())),
            transform,
            MeshMaterial3d(material_handle.clone()),
        ))
        .id();

    // Create slides.
    let slide0 = commands
        .entity(cube)
        .act(field!(<Transform>::scale), |_| Vec3::ONE)
        .with_ease(ease::cubic::ease_out)
        .play(1.0);

    let slide1 = [
        [
            commands
                .entity(cube)
                .act(field!(<Transform>::translation::x), move |_| {
                    -X_OFFSET
                })
                .with_ease(ease::cubic::ease_out)
                .play(1.0),
            commands
                .entity(cube)
                .act(
                    field!(<StandardMaterial>::base_color),
                    move |_| palettes::tailwind::ZINC_700.into(),
                )
                .with_ease(ease::cubic::ease_out)
                .play(1.0),
        ]
        .all(),
        commands
            .entity(sphere)
            .act(field!(<Transform>::scale), |_| Vec3::ONE)
            .with_ease(ease::cubic::ease_out)
            .play(1.0),
    ]
    .flow(0.1);

    commands.create_timeline([slide0, slide1]);
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
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn slide_movement(
    mut q_timelines: Query<(&mut Timeline, &mut TimelinePlayback)>,
    q_sequences: Query<(&Sequence, &SequenceController)>,
    keys: Res<ButtonInput<KeyCode>>,
) -> Result {
    for (mut timeline, mut playback) in q_timelines.iter_mut() {
        if keys.just_pressed(KeyCode::ArrowRight) {
            // Move to the start of the next sequence in
            // normal circumstances.
            //
            // However, if we're already at the last
            // sequence, move towards the end.

            if timeline.is_last_sequence() {
                timeline.insert_command(TimelineCommand::Current(
                    SequencePoint::End,
                ));
            } else {
                timeline.insert_command(TimelineCommand::Next(
                    SequencePoint::Start,
                ));
            }

            playback.pause();
        } else if keys.just_pressed(KeyCode::ArrowLeft) {
            // Move to the start of the previous sequence.
            timeline.insert_command(TimelineCommand::Previous(
                SequencePoint::Start,
            ));

            playback.pause();
        } else if keys.just_pressed(KeyCode::Space) {
            let (sequence, controller) = timeline
                .curr_sequence_id()
                .and_then(|e| q_sequences.get(e).ok())
                .ok_or("Can't get sequence.")?;

            if keys.pressed(KeyCode::ShiftLeft) {
                // Already reached the start. Go to the previous sequence.
                if controller.curr_time() <= 0.0 {
                    timeline.insert_command(
                        TimelineCommand::Previous(SequencePoint::End),
                    );
                }
                playback.backward();
            } else {
                // Already reached the end. Go to the next sequence.
                if controller.curr_time() >= sequence.duration() {
                    timeline.insert_command(TimelineCommand::Next(
                        SequencePoint::Start,
                    ));
                }

                playback.forward();
            }
        } else if keys.just_pressed(KeyCode::Escape) {
            playback.pause();
        }
    }

    Ok(())
}
