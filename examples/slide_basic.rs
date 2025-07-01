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
        .add_systems(Startup, (setup, slide_basic))
        .add_systems(Update, slide_movement)
        .add_observer(forward_motion)
        .add_observer(backward_motion)
        .run();
}

fn slide_basic(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Color.
    let green = Srgba::hex("A9DC76").unwrap().into();
    let blue = Srgba::hex("78DCE8").unwrap().into();
    let base0 = Srgba::hex("19181A").unwrap().into();

    // Cube.
    let x_offset = 2.0;
    let transform = Transform::default().with_scale(Vec3::splat(0.0));
    let material = StandardMaterial {
        base_color: green,
        ..default()
    };
    let material_handle = materials.add(material.clone());
    let cube = commands
        .spawn((
            NotShadowCaster,
            Mesh3d(meshes.add(Cuboid::default())),
            transform,
            MeshMaterial3d(material_handle.clone()),
        ))
        .id();

    // Sphere.
    let transform = Transform::default()
        .with_translation(Vec3::X * x_offset)
        .with_scale(Vec3::splat(0.0));
    let material = StandardMaterial {
        base_color: blue,
        ..default()
    };
    let material_handle = materials.add(material.clone());
    let sphere = commands
        .spawn((
            NotShadowCaster,
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
                    -x_offset
                })
                .with_ease(ease::cubic::ease_out)
                .play(1.0),
            commands
                .entity(cube)
                .act(
                    field!(<StandardMaterial>::base_color),
                    move |_| base0,
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
            ..default()
        },
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 15.0),
        Tonemapping::AcesFitted,
        bevy::core_pipeline::bloom::Bloom::default(),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn slide_movement(
    mut commands: Commands,
    mut q_timelines: Query<(&mut TimelinePlayback, Entity)>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for (mut playback, entity) in q_timelines.iter_mut() {
        if keys.just_pressed(KeyCode::Space) {
            if keys.pressed(KeyCode::ShiftLeft) {
                // Backward motion.
                commands.entity(entity).trigger(BackwardMotion);
            } else {
                // Forward motion.
                commands.entity(entity).trigger(ForwardMotion);
            }
        }

        if keys.just_pressed(KeyCode::Escape) {
            playback.pause();
        }
    }
}

fn forward_motion(
    trigger: Trigger<ForwardMotion>,
    mut commands: Commands,
    mut q_timelines: Query<(&Timeline, &mut TimelinePlayback)>,
) -> Result {
    let entity = trigger.target();
    let (timeline, mut playback) = q_timelines.get_mut(entity)?;

    // Forward motion.
    match *playback {
        TimelinePlayback::Forward => {
            // Jump to the end of the sequence.
            commands.entity(entity).trigger(JumpSequence {
                index: timeline.sequence_index(),
                playback: TimelinePlayback::Pause,
                point: SequencePoint::End,
            });
        }
        TimelinePlayback::Backward => {
            // Switch to playing forward.
            playback.forward();
        }
        TimelinePlayback::Pause => {
            match timeline.sequence_point() {
                SequencePoint::End => {
                    // We're already in the last sequence.
                    if timeline.is_last_sequence() {
                        return Ok(());
                    }

                    // Move to the next sequence and start playing.
                    let jump = JumpSequence {
                        index: timeline.sequence_index() + 1,
                        playback: TimelinePlayback::Forward,
                        point: SequencePoint::Start,
                    };

                    commands.entity(entity).trigger(jump);
                }
                _ => {
                    // Switch to playing forward.
                    playback.forward();
                }
            }
        }
    }

    Ok(())
}

fn backward_motion(
    trigger: Trigger<BackwardMotion>,
    mut commands: Commands,
    mut q_timelines: Query<(&Timeline, &mut TimelinePlayback)>,
) -> Result {
    let entity = trigger.target();
    let (timeline, mut playback) = q_timelines.get_mut(entity)?;

    // Backward motion.
    match *playback {
        TimelinePlayback::Forward => {
            // Switch to playing backward.
            playback.backward();
        }
        TimelinePlayback::Backward => {
            // Jump to the start of the sequence.
            commands.entity(entity).trigger(JumpSequence {
                index: timeline.sequence_index(),
                playback: TimelinePlayback::Pause,
                point: SequencePoint::Start,
            });
        }
        TimelinePlayback::Pause => {
            match timeline.sequence_point() {
                SequencePoint::Start => {
                    // We're already in the first sequence.
                    if timeline.is_first_sequence() {
                        return Ok(());
                    }

                    // Move to the previous sequence and start playing.
                    let jump = JumpSequence {
                        index: timeline
                            .sequence_index()
                            .saturating_sub(1),
                        playback: TimelinePlayback::Backward,
                        point: SequencePoint::End,
                    };

                    commands.entity(entity).trigger(jump);
                }
                _ => {
                    // Switch to playing backward.
                    playback.backward();
                }
            }
        }
    }

    Ok(())
}

#[derive(Event)]
pub struct ForwardMotion;

#[derive(Event)]
pub struct BackwardMotion;
