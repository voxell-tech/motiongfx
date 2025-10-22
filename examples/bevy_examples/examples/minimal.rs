use bevy::prelude::*;
use bevy_motiongfx::BevyMotionGfxPlugin;
use bevy_motiongfx::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BevyMotionGfxPlugin))
        .add_systems(Startup, (setup, build_timeline))
        .run();
}

/// Spawns the camera and the directional light.
fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 15.0),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Creates the timeline and plays it.
fn build_timeline(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material =
        materials.add(StandardMaterial::from_color(Srgba::BLUE));
    // Spawns the cube.
    let cube = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(-3.0, 0.0, 0.0),
        ))
        .id();

    // Build the timeline.
    let mut b = TimelineBuilder::new();
    let track = [
        b.act_interp(
            cube,
            field!(<Transform>::translation::x),
            |x| x + 6.0,
        )
        .play(1.0),
        b.act_interp(
            material.untyped().id(),
            field!(<StandardMaterial>::base_color),
            |_| Srgba::RED.into(),
        )
        .play(1.0),
    ]
    .ord_all()
    .compile();

    b.add_tracks(track);
    let timeline = b.compile();

    // Spawns the timeline and start playing.
    commands
        .spawn((timeline, RealtimePlayer::new().with_playing(true)));
}
