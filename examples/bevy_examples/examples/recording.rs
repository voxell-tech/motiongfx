use core::f32::consts::FRAC_PI_2;

use bevy::asset::RenderAssetUsages;
use bevy::color::palettes;
use bevy::prelude::*;
use bevy::render::{
    gpu_readback::{Readback, ReadbackComplete},
    render_resource::{Extent3d, TextureDimension, TextureUsages},
    view::screenshot::{Screenshot, save_to_disk},
};
use bevy_examples::timeline_movement;
use bevy_motiongfx::{
    BevyMotionGfxPlugin, controller::RecordPlayer, prelude::*,
    world::TimelineComplete,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BevyMotionGfxPlugin))
        .add_systems(PreStartup, spawn_canvas)
        .add_systems(Startup, (setup, spawn_timeline))
        .add_observer(save_frame)
        .add_systems(Update, timeline_movement)
        .run();
}

#[derive(Resource)]
struct RecordingCanvas(Handle<Image>);

fn spawn_canvas(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let mut image = Image::new_fill(
        Extent3d {
            width: 3840,
            height: 2160,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );

    image.texture_descriptor.usage =
    // COPY_SRC allows the GPU to write onto the Image buffer
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC;

    let handle = images.add(image);

    commands.insert_resource(RecordingCanvas(handle.clone()));
    commands.spawn(Readback::texture(handle));
}

fn save_frame(
    gpu_img: On<ReadbackComplete>,
    mut commands: Commands,
    readbacks: Query<&Readback>,
    player: Query<&RecordPlayer, Without<TimelineComplete>>,
    canvas: Res<RecordingCanvas>,
) {
    let Ok(player) = player.single() else {
        return;
    };

    if let Readback::Texture(handle) =
        readbacks.get(gpu_img.entity).unwrap()
    {
        if *handle != canvas.0 {
            return;
        }
        commands.spawn(Screenshot::image(handle.clone())).observe(
            save_to_disk(format!(
                "frames/frame_{:05}.png",
                player.curr_frame
            )),
        );
    }
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
        .act(cube, field!(<Transform>::translation), |x| {
            x + Vec3::ZERO.with_x(10.0).with_z(1.0)
        })
        .with_interp(|start, end, t| arc_lerp_3d(*start, *end, t))
        .with_ease(ease::cubic::ease_in_out)
        .play(1.0)
        .compile();

    b.add_tracks(track);

    commands.spawn((
        motiongfx.add_timeline(b.compile()),
        RecordPlayer::default().with_fps(30),
    ));
}

fn setup(mut commands: Commands, image: Res<RecordingCanvas>) {
    commands
        .spawn((
            Camera {
                clear_color: Color::BLACK.into(),
                ..default()
            },
            Camera3d::default(),
            // Top down view.
            Transform::from_xyz(0.0, 18.0, 0.0)
                .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        ))
        .with_child((
            Camera {
                clear_color: Color::BLACK.into(),
                order: 1,
                ..Default::default()
            },
            bevy::camera::RenderTarget::Image(
                bevy::camera::ImageRenderTarget {
                    handle: image.0.clone(),
                    scale_factor: 1.0,
                },
            ),
            Camera3d::default(),
        ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

// TODO: Optimize this.
pub fn arc_lerp_3d(start: Vec3, end: Vec3, t: f32) -> Vec3 {
    let center = (start + end) * 0.5;

    let start_dir = Dir3::new(start - center);
    let end_dir = Dir3::new(end - center);

    let (Ok(start_dir), Ok(end_dir)) = (start_dir, end_dir) else {
        // Revert to linear interpolation.
        return start.lerp(end, t);
    };

    let target_dir = start_dir.slerp(end_dir, t);

    center + target_dir.as_vec3() * (center - start).length()
}
