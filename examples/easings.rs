use bevy::core_pipeline::bloom::Bloom;
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

    // Colors.
    let blue =
        LinearRgba::from(Srgba::hex("78DCE8").unwrap()) * 100.0;
    let red = LinearRgba::from(Srgba::hex("FF6188").unwrap()) * 100.0;

    // Create spheres.
    let mut spheres = Vec::with_capacity(capacity);
    let mesh_handle = meshes.add(Sphere::default());
    let material = StandardMaterial {
        base_color: Color::WHITE,
        emissive: blue,
        ..default()
    };

    for i in 0..capacity {
        let material_handle = materials.add(material.clone());
        let transform = Transform::from_translation(Vec3::new(
            -5.0,
            (i as f32) - (capacity as f32) * 0.5,
            0.0,
        ))
        .with_scale(Vec3::ONE);

        let id = commands
            .spawn((
                NotShadowCaster,
                Mesh3d(mesh_handle.clone()),
                transform,
                MeshMaterial3d(material_handle.clone()),
            ))
            .id();

        spheres.push(id);
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

    commands.create_timeline([sequence]);
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
        Bloom::default(),
    ));
}

fn timeline_movement(
    mut q_timelines: Query<&mut Timeline>,
    keys: Res<ButtonInput<KeyCode>>,
    mut is_playing: Local<bool>,
) {
    for mut timeline in q_timelines.iter_mut() {
        if *is_playing == false {
            timeline.pause();
        }

        if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
            timeline.play_forward(1.0);
        }

        if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
            timeline.play_backward(1.0);
        }

        if keys.just_pressed(KeyCode::Space) {
            *is_playing = true;
            if keys.pressed(KeyCode::ShiftLeft) {
                timeline.play_backward(1.0);
            } else {
                timeline.play_forward(1.0);
            }
        }

        if keys.just_pressed(KeyCode::Escape) {
            *is_playing = false;
        }
    }
}
