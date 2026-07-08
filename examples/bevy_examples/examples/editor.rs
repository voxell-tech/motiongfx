use bevy::camera::{Hdr, Viewport};
use bevy::color::palettes;
use bevy::feathers::{
    FeathersPlugins, controls::FeathersSlider,
    dark_theme::create_dark_theme, theme::UiTheme,
};
use bevy::prelude::*;
use bevy::ui_widgets::{
    Slider, SliderPrecision, SliderStep, TrackClick, ValueChange,
    slider_self_update,
};
use bevy::window::WindowResized;
use bevy_motiongfx::{
    BevyMotionGfxPlugin, controller::PassivePlayer, prelude::*,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            BevyMotionGfxPlugin,
            FeathersPlugins,
        ))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, (setup, spawn_timeline).chain())
        .add_systems(Update, (slide_movement, set_camera_viewports))
        .add_observer(add_timline_bar_on_new_timeline)
        .run();
}

fn spawn_timeline(
    mut commands: Commands,
    mut motiongfx: ResMut<MotionGfxManager>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const X_OFFSET: f32 = 2.0;

    // Spawn 3d models.
    let cube_mat_handle = materials.add(
        StandardMaterial::from_color(palettes::tailwind::LIME_200),
    );
    let cube_mat_id = cube_mat_handle.id().untyped();
    let cube = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            Transform::default().with_scale(Vec3::splat(0.0)),
            MeshMaterial3d(cube_mat_handle),
        ))
        .id();

    let sphere = commands
        .spawn((
            Mesh3d(meshes.add(Sphere::default())),
            Transform::default()
                .with_translation(Vec3::X * X_OFFSET)
                .with_scale(Vec3::splat(0.0)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: palettes::tailwind::CYAN_300.into(),
                ..default()
            })),
        ))
        .id();

    // Build the timeline.
    let mut b = motiongfx.create_builder();

    // Generate slide sequences.
    let slide0 = b
        .act(cube, path!(<Transform>::scale), |_| Vec3::ONE)
        .with_ease(ease::cubic::ease_out)
        .play(1.0)
        .compile();

    let slide1 = [
        [
            b.act(
                cube,
                path!(<Transform>::translation::x),
                move |_| -X_OFFSET,
            )
            .with_ease(ease::cubic::ease_out)
            .play(1.0),
            b.act(
                cube_mat_id,
                path!(<StandardMaterial>::base_color),
                move |_| palettes::tailwind::ZINC_700.into(),
            )
            .with_ease(ease::cubic::ease_out)
            .play(1.0),
        ]
        .ord_all(),
        b.act(sphere, path!(<Transform>::scale), |_| Vec3::ONE)
            .with_ease(ease::cubic::ease_out)
            .play(1.0),
    ]
    .ord_flow(0.1)
    .compile();

    b.add_tracks([slide0, slide1]);

    let timeline = b.compile();

    commands.spawn((
        motiongfx.add_timeline(timeline),
        PassivePlayer::default(),
    ));
}

#[derive(Component, Clone, FromTemplate)]
#[relationship(relationship_target = View)]
pub struct ViewOf(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = ViewOf, linked_spawn)]
pub struct View(Entity);

#[derive(Component)]
struct ViewportPosition {
    size: u32,
}

/// Marker Component for TimelineUi element
#[derive(SceneComponent, Default, Clone)]
struct TimelineWindow;

impl TimelineWindow {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                height: percent(100.),
                width: percent(100.),
            }
            TimelineWindow
        }
    }
}

fn add_timline_bar_on_new_timeline(
    event: On<Add, TimelineId>,
    motiongfx: Res<MotionGfxManager>,
    timelines: Query<&TimelineId>,
    mut commands: Commands,
    window: Single<Entity, With<TimelineWindow>>,
) {
    let entity = event.entity;

    let id = timelines.get(entity).expect("TimelineId should exist");
    let tracks = motiongfx
        .get_timeline(id)
        .expect("Timeline should exist")
        .tracks();

    let window = window.entity();

    commands.spawn_scene(bsn! {
        @TimelineBar {
            @tracks: tracks
        }
        ChildOf(window)
        ViewOf(entity)
    });
}

#[derive(Default)]
struct TimelineProps {
    tracks: Box<[Track]>,
}

#[derive(SceneComponent, Default, Clone)]
#[scene(TimelineProps)]
struct TimelineBar;

impl TimelineBar {
    fn scene(tracks: TimelineProps) -> impl Scene {
        // TODO extract more track info: Timelines, Action names
        let _num_track = tracks.tracks.len();

        let duration = tracks
            .tracks
            .iter()
            .fold(0.0, |acc, track| acc + track.duration());
        bsn! {
                @FeathersSlider {
                    @max: duration,
                    @value: 0.0,
                }
                SliderStep(10.)
                SliderPrecision(2)
                Slider {
                    track_click: TrackClick::Snap
                }
                // TODO ViewOf relationship should be in here
                on(slider_self_update)
                on(player_update)
        }
    }
}

fn player_update(
    value_change: On<ValueChange<f32>>,
    mut players: Query<&mut PassivePlayer>,
    timebars: Query<&ViewOf, With<TimelineBar>>,
) {
    let timebar = timebars
        .get(value_change.source)
        .expect("Should be the entity with ViewOf relationship");

    let mut player = players.get_mut(timebar.0).unwrap();

    player.set_time(value_change.value);

    // Get player with the same entity as this entity's ViewOf relation
}

fn setup(mut commands: Commands) {
    let ui_camera = commands
        .spawn((
            Camera2d::default(),
            Camera {
                order: 10,
                ..default()
            },
            ViewportPosition { size: 4 },
        ))
        .id();

    commands.spawn((
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
        Camera3d::default(),
        Hdr,
        Transform::from_xyz(0.0, 0.0, 15.0),
    ));
    commands
        .spawn_scene(bsn! {@TimelineWindow})
        .insert(UiTargetCamera(ui_camera));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn set_camera_viewports(
    windows: Query<&Window>,
    mut window_resized_reader: MessageReader<WindowResized>,
    mut query: Query<(&ViewportPosition, &mut Camera)>,
) {
    // We need to dynamically resize the camera's viewports whenever the window size changes
    // so then each camera always takes up half the screen.
    // A resize_event is sent when the window is first created, allowing us to reuse this system for initial setup.
    for window_resized in window_resized_reader.read() {
        let window = windows.get(window_resized.window).unwrap();
        let size = window.physical_size();

        for (camera_position, mut camera) in &mut query {
            let viewport_y = size.y * (camera_position.size - 1)
                / camera_position.size;
            let viewport_height = size.y / camera_position.size;
            camera.viewport = Some(Viewport {
                physical_position: UVec2 {
                    x: 0,
                    y: viewport_y,
                },
                physical_size: size.with_y(viewport_height),
                ..default()
            });
        }
    }
}
fn slide_movement(
    mut motiongfx: ResMut<MotionGfxManager>,
    mut q_timelines: Query<&mut PassivePlayer>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for mut player in q_timelines.iter_mut() {
        if keys.just_pressed(KeyCode::ArrowRight) {
            // Move to the start of the next track.
            let target_index = player.get_track() + 1;
            player.set_track_index(target_index);
            player.set_time(0.0);

            // player.set_playing(false);
        }

        if keys.just_pressed(KeyCode::ArrowLeft) {
            // Move to the start of the previous track.
            let target_index = player.get_track().saturating_sub(1);
            player.set_track_index(target_index);
            player.set_time(0.0);

            // player.set_playing(false);
        }

        // if keys.just_pressed(KeyCode::Space) {
        //     if keys.any_pressed([
        //         KeyCode::ShiftLeft,
        //         KeyCode::ShiftRight,
        //     ]) {
        //         player.set_playing(true).set_time_scale(-1.0);

        //         if player.curr_time() <= 0.0
        //             && player.curr_index() > 0
        //         {
        //             // Move to the end of the previous track.
        //             let target_index =
        //                 player.curr_index().saturating_sub(1);
        //             player.set_track(target_index);
        //             player.set_time(f32::MAX);
        //         }
        //     } else {
        //         player.set_playing(true).set_time_scale(1.0);

        //         if player.is_track_end() && !player.is_complete() {
        //             // Move to the start of the next track.
        //             let target_index = player.curr_index() + 1;
        //             player.set_track(target_index);
        //             player.set_time(0.0);
        //         }
        //     }
        // }

        // if keys.just_pressed(KeyCode::Escape) {
        //     player.set_playing(false);
        // }
    }
}
