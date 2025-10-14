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
        // Custom plugins
        .add_plugins((MotionGfx, MotionGfxBevy))
        .add_systems(Startup, (setup_system, easings_system))
        .add_systems(Update, timeline_movement_system)
        .run();
}

fn easings_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const CAPACITY: usize = 10;

    // Color palette
    let palette = ColorPalette::default();

    let mut spheres = Vec::with_capacity(CAPACITY);
    // States
    let mut transform_motions = Vec::with_capacity(CAPACITY);
    let mut material_motions = Vec::with_capacity(CAPACITY);

    // Create sphere objects (Entity)
    let material = StandardMaterial {
        base_color: Color::WHITE,
        emissive: *palette.get_or_default(&ColorKey::Blue) * 100.0,
        ..default()
    };

    for i in 0..CAPACITY {
        let transform =
            Transform::from_translation(Vec3::new(-5.0, (i as f32) - (CAPACITY as f32) * 0.5, 0.0))
                .with_scale(Vec3::ONE);

        let sphere = commands
            .spawn(PbrBundle {
                transform,
                mesh: meshes.add(Sphere::default()),
                material: materials.add(material.clone()),
                ..default()
            })
            .insert(NotShadowCaster)
            .id();

        transform_motions.push(TransformMotion::new(sphere, transform));
        material_motions.push(StandardMaterialMotion::new(sphere, material.clone()));

        spheres.push(sphere);
    }

    // Generate easing animations
    let mut easing_seqs = Vec::with_capacity(CAPACITY);

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

    for i in 0..CAPACITY {
        easing_seqs.push(
            all(&[
                commands.play(transform_motions[i].translate_add(Vec3::X * 10.0), 1.0),
                commands.play(
                    material_motions[i]
                        .emissive_to(*palette.get_or_default(&ColorKey::Red) * 100.0),
                    1.0,
                ),
            ])
            .with_ease(easings[i]),
        );
    }

    let sequence = chain(&easing_seqs);

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
            transform: Transform::from_xyz(0.0, 0.0, 15.0),
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
