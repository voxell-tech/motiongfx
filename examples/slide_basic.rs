use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use bevy_motiongfx::prelude::*;

fn main() {
    App::new()
        // Bevy plugins
        .add_plugins(DefaultPlugins)
        // Custom plugins
        .add_plugins(bevy_motiongfx::MotionGfxPlugin)
        .add_systems(Startup, (setup, slide_basic))
        .add_systems(Update, slide_movement)
        .run();
}

fn slide_basic(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Color palette
    let palette = ColorPalette::default();

    // Cube
    let x_offset = 2.0;
    let transform = Transform::default().with_scale(Vec3::splat(0.0));
    let material = StandardMaterial {
        base_color: palette.get(ColorKey::Green),
        ..default()
    };
    let material_handle = materials.add(material.clone());
    let id = commands
        .spawn((
            NotShadowCaster,
            Mesh3d(meshes.add(Cuboid::default())),
            transform,
            MeshMaterial3d(material_handle.clone()),
        ))
        .id();
    let mut cube = (id, (transform, material));

    // Sphere
    let transform = Transform::default()
        .with_translation(Vec3::X * x_offset)
        .with_scale(Vec3::splat(0.0));
    let material = StandardMaterial {
        base_color: palette.get(ColorKey::Blue),
        ..default()
    };
    let material_handle = materials.add(material.clone());
    let id = commands
        .spawn((
            NotShadowCaster,
            Mesh3d(meshes.add(Sphere::default())),
            transform,
            MeshMaterial3d(material_handle.clone()),
        ))
        .id();
    let mut sphere = (id, (transform, material));

    // Create slides
    let slide0 = commands.play_motion(
        cube.transform().to_scale(Vec3::ONE).animate(1.0),
    );

    let slide1 = [
        commands
            .add_motion(
                cube.transform()
                    .to_translation_x(-x_offset)
                    .animate(1.0),
            )
            .add_motion(
                cube.std_material()
                    .to_base_color(palette.get(ColorKey::Base0))
                    .animate(1.0),
            )
            .all(),
        commands.play_motion(
            sphere.transform().to_scale(Vec3::ONE).animate(1.0),
        ),
    ]
    .flow(0.1);

    commands.spawn(create_slide(vec![slide0, slide1]));
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
        bevy::core_pipeline::bloom::Bloom::default(),
    ));

    // Directional light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn slide_movement(
    mut q_slides: Query<&mut SlideController>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for mut slide in q_slides.iter_mut() {
        if keys.just_pressed(KeyCode::Space) {
            slide.set_time_scale(1.0);

            if keys.pressed(KeyCode::ShiftLeft) {
                slide.prev();
            } else {
                slide.next();
            }
        }

        if keys.just_pressed(KeyCode::Escape) {
            slide.set_time_scale(0.0);
        }
    }
}
