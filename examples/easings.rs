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
        .add_systems(Startup, (setup, easings))
        .add_systems(Update, timeline_movement_system)
        .run();
}

pub fn easings(
    mut commands: Commands,
    mut sequence: ResMut<Sequence>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const CAPACITY: usize = 10;

    let mut spheres: Vec<Entity> = Vec::with_capacity(CAPACITY);
    // States
    let mut cube_translations: Vec<Translation> = Vec::with_capacity(CAPACITY);
    let mut cube_colors: Vec<BaseColor> = Vec::with_capacity(CAPACITY);

    // Create cube objects (Entity)
    let material: StandardMaterial = StandardMaterial {
        base_color: style::BLUE.into(),
        ..default()
    };

    for i in 0..CAPACITY {
        let transform: Transform = Transform::from_translation(Vec3::new(
            -10.0,
            (i as f32) - (CAPACITY as f32) * 0.5,
            0.0,
        ))
        .with_scale(Vec3::ONE * 0.48);

        let cube = commands
            .spawn(PbrBundle {
                transform,
                mesh: meshes.add(shape::UVSphere::default().into()),
                material: materials.add(material.clone()),
                ..default()
            })
            .insert(NotShadowCaster)
            .id();

        cube_translations.push(Translation::from_transform(cube, &transform));
        cube_colors.push(BaseColor::from_material(cube, &material));

        spheres.push(cube);
    }

    let mut act: ActionBuilder = ActionBuilder::new(&mut commands);

    // Generate cube animations
    // Actions
    let mut cube_actions: Vec<ActionMetaGroup> = Vec::with_capacity(CAPACITY);

    let easings: [ease::EaseFn; CAPACITY] = [
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
        cube_actions.push(
            all(&[
                act.play(cube_translations[i].translate(Vec3::X * 20.0), 1.0),
                act.play(cube_colors[i].color_to(style::RED), 1.0),
            ])
            .with_ease(easings[i]),
        );
    }

    sequence.play(chain(&cube_actions));
}

fn setup(mut commands: Commands) {
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
    mut timeline: ResMut<Timeline>,
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    if keys.pressed(KeyCode::D) {
        timeline.target_time += time.delta_seconds();
    }

    if keys.pressed(KeyCode::A) {
        timeline.target_time -= time.delta_seconds();
    }

    if keys.pressed(KeyCode::Space) && keys.pressed(KeyCode::ShiftLeft) {
        timeline.time_scale = -1.0;
        timeline.is_playing = true;
    } else if keys.pressed(KeyCode::Space) {
        timeline.time_scale = 1.0;
        timeline.is_playing = true;
    }
}
