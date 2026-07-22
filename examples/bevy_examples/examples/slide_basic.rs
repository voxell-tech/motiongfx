use std::time::Duration;

use bevy::camera::Hdr;
use bevy::color::palettes;
use bevy::prelude::*;
use bevy_motiongfx::BevyMotionGfxPlugin;
use bevy_motiongfx::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BevyMotionGfxPlugin))
        .add_systems(Startup, (setup, spawn_timeline))
        .add_systems(Update, slide_movement)
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
        .play(s(1))
        .compile();

    let slide1 = [
        [
            b.act(
                cube,
                path!(<Transform>::translation::x),
                move |_| -X_OFFSET,
            )
            .with_ease(ease::cubic::ease_out)
            .play(s(1)),
            b.act(
                cube_mat_id,
                path!(<StandardMaterial>::base_color),
                move |_| palettes::tailwind::ZINC_700.into(),
            )
            .with_ease(ease::cubic::ease_out)
            .play(s(1)),
        ]
        .ord_all(),
        b.act(sphere, path!(<Transform>::scale), |_| Vec3::ONE)
            .with_ease(ease::cubic::ease_out)
            .play(s(1)),
    ]
    .ord_flow(cs(10))
    .compile();

    b.add_tracks([slide0, slide1]);

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
        Hdr,
        Transform::from_xyz(0.0, 0.0, 15.0),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn slide_movement(
    mut motiongfx: ResMut<MotionGfxManager>,
    mut q_timelines: Query<(&TimelineId, &mut RealtimePlayer)>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for (id, mut player) in q_timelines.iter_mut() {
        if let Some(timeline) = motiongfx.get_timeline_mut(id) {
            if keys.just_pressed(KeyCode::ArrowRight) {
                // Move to the start of the next track.
                let target_index = timeline.curr_index() + 1;
                timeline.set_target_track(target_index);
                timeline.set_target_time(s(0));

                player.set_playing(false);
            }

            if keys.just_pressed(KeyCode::ArrowLeft) {
                // Move to the start of the previous track.
                let target_index =
                    timeline.curr_index().saturating_sub(1);
                timeline.set_target_track(target_index);
                timeline.set_target_time(s(0));

                player.set_playing(false);
            }

            if keys.just_pressed(KeyCode::Space) {
                if keys.any_pressed([
                    KeyCode::ShiftLeft,
                    KeyCode::ShiftRight,
                ]) {
                    player.set_playing(true).set_time_scale(-1.0);

                    if timeline.curr_time() == Duration::ZERO
                        && timeline.curr_index() > 0
                    {
                        // Move to the end of the previous track.
                        let target_index =
                            timeline.curr_index().saturating_sub(1);
                        timeline.set_target_track(target_index);
                        // Clamped to the new track's duration.
                        timeline.set_target_time(Duration::MAX);
                    }
                } else {
                    player.set_playing(true).set_time_scale(1.0);

                    if timeline.is_track_end()
                        && !timeline.is_complete()
                    {
                        // Move to the start of the next track.
                        let target_index = timeline.curr_index() + 1;
                        timeline.set_target_track(target_index);
                        timeline.set_target_time(s(0));
                    }
                }
            }

            if keys.just_pressed(KeyCode::Escape) {
                player.set_playing(false);
            }
        }
    }
}
