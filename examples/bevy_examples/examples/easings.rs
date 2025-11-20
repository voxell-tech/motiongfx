use bevy::color::palettes;
use bevy::light::NotShadowCaster;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::Hdr;
use bevy_examples::timeline_movement;
use bevy_motiongfx::BevyMotionGfxPlugin;
use bevy_motiongfx::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BevyMotionGfxPlugin))
        .add_systems(Startup, (setup, spawn_timeline))
        .add_systems(Update, timeline_movement)
        .run();
}

fn spawn_timeline(
    mut commands: Commands,
    mut motiongfx: ResMut<MotionGfxWorld>,
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
    let blue = LinearRgba::from(palettes::tailwind::CYAN_300) * 100.0;
    let red = LinearRgba::from(palettes::tailwind::ROSE_400) * 100.0;

    // Spawn spheres.
    let mut spheres = Vec::with_capacity(capacity);
    let mut sphere_mats = Vec::with_capacity(capacity);
    let mesh_handle = meshes.add(Sphere::default());
    let material = StandardMaterial {
        base_color: Color::WHITE,
        emissive: blue,
        ..default()
    };

    for i in 0..capacity {
        let sphere_mat = materials.add(material.clone());
        let sphere = commands
            .spawn((
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(sphere_mat.clone()),
                Transform::from_translation(Vec3::new(
                    -5.0,
                    (i as f32) - (capacity as f32) * 0.5,
                    0.0,
                ))
                .with_scale(Vec3::ONE),
                NotShadowCaster,
            ))
            .id();

        spheres.push(sphere);
        sphere_mats.push(sphere_mat.untyped().id());
    }

    // Build the timeline.
    let mut b = TimelineBuilder::new();

    let track = easings
        .into_iter()
        .enumerate()
        // .zip(easings)
        .map(|(i, ease_fn)| {
            [
                b.act_interp(
                    spheres[i],
                    field!(<Transform>::translation::x),
                    |x| x + 10.0,
                )
                .with_ease(ease_fn)
                .play(1.0),
                b.act_interp(
                    sphere_mats[i],
                    field!(<StandardMaterial>::emissive),
                    move |_| red,
                )
                .with_ease(ease_fn)
                .play(1.0),
            ]
            .ord_all()
        })
        .ord_chain();

    b.add_tracks(track.compile());

    commands.spawn((
        motiongfx.add_timeline(b.compile()),
        RealtimePlayer::new().with_playing(true),
    ));
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
        Camera3d::default(),
        Hdr,
        Transform::from_xyz(0.0, 0.0, 15.0),
        Bloom::default(),
    ));
}
