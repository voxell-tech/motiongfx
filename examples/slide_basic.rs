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
        .insert_resource(Msaa::Off)
        // Custom plugins
        .add_plugins((MotionGfx, MotionGfxBevy))
        .add_systems(Startup, (setup_system, slide_basic_system))
        .add_systems(Update, slide_movement_system)
        .run();
}

fn slide_basic_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Color palette
    let palette: ColorPalette<ColorKey> = ColorPalette::default();

    // Materials
    let green_material: StandardMaterial = StandardMaterial {
        base_color: *palette.get_or_default(&ColorKey::Green),
        ..default()
    };
    let blue_material: StandardMaterial = StandardMaterial {
        base_color: *palette.get_or_default(&ColorKey::Blue),
        ..default()
    };

    let x_offset: f32 = 2.0;
    // Cube
    let cube_pbr: PbrBundle = PbrBundle {
        transform: Transform::default().with_scale(Vec3::splat(0.0)),
        mesh: meshes.add(Cuboid::default()),
        material: materials.add(green_material.clone()),
        ..default()
    };
    let cube: Entity = commands
        .spawn(cube_pbr.clone())
        .insert(NotShadowCaster)
        .id();

    let mut cube_transform: TransformMotion = TransformMotion::new(cube, cube_pbr.transform);
    let mut cube_material: StandardMaterialMotion =
        StandardMaterialMotion::new(cube, green_material);

    // Sphere
    let sphere_pbr: PbrBundle = PbrBundle {
        transform: Transform::default()
            .with_translation(Vec3::X * x_offset)
            .with_scale(Vec3::splat(0.0)),
        mesh: meshes.add(Sphere::default()),
        material: materials.add(blue_material),
        ..default()
    };
    let sphere: Entity = commands
        .spawn(sphere_pbr.clone())
        .insert(NotShadowCaster)
        .id();

    let mut sphere_tranform: TransformMotion = TransformMotion::new(sphere, sphere_pbr.transform);

    // Create slides
    let slide0: Sequence = commands
        .play(cube_transform.scale_to(Vec3::ONE), 1.0)
        .with_ease(ease::cubic::ease_in_out);

    let slide1: Sequence = flow(
        0.1,
        &[
            all(&[
                commands.play(cube_transform.translate_add(Vec3::X * -x_offset), 1.0),
                commands.play(
                    cube_material.base_color_to(*palette.get_or_default(&ColorKey::Base0)),
                    1.0,
                ),
            ]),
            commands.play(sphere_tranform.scale_to(Vec3::ONE), 1.0),
        ],
    )
    .with_ease(ease::cubic::ease_in_out);

    commands.spawn(create_slide(vec![slide0, slide1]));
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

fn slide_movement_system(
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
