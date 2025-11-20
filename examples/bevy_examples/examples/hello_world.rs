use bevy::color::palettes;
use bevy::light::NotShadowCaster;
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
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    const WIDTH: usize = 10;
    const HEIGHT: usize = 10;

    const CAPACITY: usize = WIDTH * HEIGHT;

    // Spawn cubes.
    let mut cubes = Vec::with_capacity(CAPACITY);
    let mesh_handle = meshes.add(Cuboid::default());
    let material_handle = materials.add(StandardMaterial {
        base_color: palettes::tailwind::LIME_200.into(),
        ..default()
    });

    for w in 0..WIDTH {
        for h in 0..HEIGHT {
            let transform = Transform::from_translation(Vec3::new(
                (w as f32) - (WIDTH as f32) * 0.5 - 1.0,
                (h as f32) - (HEIGHT as f32) * 0.5,
                0.0,
            ))
            .with_scale(Vec3::ZERO);
            let id = commands
                .spawn((
                    NotShadowCaster,
                    Mesh3d(mesh_handle.clone()),
                    transform,
                    MeshMaterial3d(material_handle.clone()),
                ))
                .id();
            cubes.push(id);
        }
    }

    // Build the timeline.
    let mut b = TimelineBuilder::new();
    let mut cube_tracks = Vec::with_capacity(CAPACITY);

    for w in 0..WIDTH {
        for h in 0..HEIGHT {
            let c = w * WIDTH + h;
            let cube = cubes[c];

            let circ_ease = ease::circ::ease_in_out;

            let track = [
                b.act_interp(
                    cube,
                    field!(<Transform>::scale),
                    |_| Vec3::splat(0.9),
                )
                .with_ease(circ_ease)
                .play(1.0),
                b.act_interp(
                    cube,
                    field!(<Transform>::translation::x),
                    |x| x + 1.0,
                )
                .with_ease(circ_ease)
                .play(1.0),
                b.act_interp(
                    cube,
                    field!(<Transform>::rotation),
                    |_| {
                        Quat::from_euler(
                            EulerRot::XYZ,
                            0.0,
                            f32::to_radians(90.0),
                            0.0,
                        )
                    },
                )
                .with_ease(circ_ease)
                .play(1.0),
            ]
            .ord_all();

            cube_tracks.push(track);
        }
    }

    let track = cube_tracks.ord_flow(0.01).compile();
    b.add_tracks(track);
    let timeline = b.compile();

    commands.spawn((
        motiongfx.add_timeline(timeline),
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
        Transform::from_xyz(0.0, 0.0, 15.0),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
