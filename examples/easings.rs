use bevy::{
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
    pbr::NotShadowCaster,
    prelude::*,
};
use bevy_motiongfx::{prelude::*, MotionGfxPlugin};

fn main() {
    App::new()
        // Bevy plugins
        .add_plugins(DefaultPlugins)
        // Custom plugins
        .add_plugins(MotionGfxPlugin)
        .add_systems(Startup, (setup, easings))
        .add_systems(Update, timeline_movement)
        .run();
}

fn easings(
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

    // Color palette
    let palette = ColorPalette::default();

    // Create spheres
    let mut spheres = Vec::with_capacity(capacity);
    let mesh_handle = meshes.add(Sphere::default());
    let material = StandardMaterial {
        base_color: Color::WHITE,
        emissive: palette.get(ColorKey::Blue).to_linear() * 100.0,
        ..default()
    };
    let material_handle = materials.add(material.clone());

    for i in 0..capacity {
        let transform =
            Transform::from_translation(Vec3::new(-5.0, (i as f32) - (capacity as f32) * 0.5, 0.0))
                .with_scale(Vec3::ONE);

        let id = commands
            .spawn((
                NotShadowCaster,
                Mesh3d(mesh_handle.clone()),
                transform,
                MeshMaterial3d(material_handle.clone()),
            ))
            .id();

        spheres.push((id, (transform, material.clone())));
    }

    // Generate sequence
    let sequence = spheres
        .iter_mut()
        .zip(easings)
        .map(|(s, e)| {
            commands
                .add_motion({
                    let x = s.transform().transform.translation.x;
                    s.transform()
                        .to_translation_x(x + 10.0)
                        .with_ease(e)
                        .animate(1.0)
                })
                .add_motion(
                    s.std_material()
                        .to_emissive(palette.get(ColorKey::Red).to_linear() * 100.0)
                        .animate(1.0),
                )
                .all()
        })
        .collect::<Vec<_>>()
        .chain();

    commands.spawn(SequencePlayerBundle {
        sequence,
        ..default()
    });
}

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn((
        Camera {
            hdr: true,
            ..default()
        },
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 15.0),
        Tonemapping::AcesFitted,
        Bloom::default(),
    ));
}

fn timeline_movement(
    mut q_timelines: Query<(&mut SequencePlayer, &mut SequenceController)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    for (mut sequence_player, mut sequence_time) in q_timelines.iter_mut() {
        if keys.pressed(KeyCode::KeyD) {
            sequence_time.target_time += time.delta_secs();
        }

        if keys.pressed(KeyCode::KeyA) {
            sequence_time.target_time -= time.delta_secs();
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
