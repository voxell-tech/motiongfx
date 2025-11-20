use bevy::color::palettes;
use bevy::prelude::*;
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
    // Spawn cube.
    let cube = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: palettes::tailwind::LIME_200.into(),
                ..default()
            })),
            Transform::from_xyz(-5.0, 0.0, 0.0),
        ))
        .id();

    // Build the timeline.
    let mut b = TimelineBuilder::new();

    let track = b
        .act_interp(cube, field!(<Transform>::translation::x), |x| {
            x + 10.0
        })
        // A custom 10 step easing.
        .with_ease(|t| ((t * 10.0) as u32) as f32 / 10.0)
        .play(1.0)
        .compile();

    b.add_tracks(track);

    commands.spawn((
        motiongfx.add_timeline(b.compile()),
        RealtimePlayer::new(),
    ));
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera {
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
