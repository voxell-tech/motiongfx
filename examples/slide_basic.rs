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

    commands.spawn(create_slide(vec![slide0, slide1]));
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
