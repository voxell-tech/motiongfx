//! Two actions on one `Transform` whose field paths overlap in memory
//! (`translation` and `translation::x`).
//!
//! - top cube: the two actions are chained, so they never overlap in
//!   time. Baking composes them: `translation::x` starts from the `x`
//!   that `translation` left, so the motion is continuous.
//! - bottom cube: the `translation::x` action overlaps `translation`
//!   in time. At compile the `translation` clip is dropped entirely,
//!   so the cube only does the x-slide and never rises.

use core::time::Duration;

use bevy::color::palettes::tailwind;
use bevy::prelude::*;
use bevy_examples::timeline_movement;
use bevy_motiongfx::BevyMotionGfxPlugin;
use bevy_motiongfx::prelude::*;

const START: Vec3 = Vec3::new(-6.0, 0.0, 0.0);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BevyMotionGfxPlugin))
        .add_systems(Startup, (setup, spawn_timeline))
        .add_systems(Update, (timeline_movement, loop_timeline))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: Color::from(tailwind::SLATE_900).into(),
            ..default()
        },
        AmbientLight {
            brightness: 400.0,
            ..default()
        },
        Transform::from_xyz(1.0, 1.0, 20.0)
            .looking_at(Vec3::new(1.0, 0.0, 0.0), Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 6_000.0,
            ..default()
        },
        Transform::from_xyz(4.0, 10.0, 8.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Text::new(
            "top: chained translation then translation::x, baking composes them (continuous)\n\
             bottom: overlapping translation::x removed the translation clip at compile\n\
             space = play,  A/Left | D/Right = scrub",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

fn spawn_cube(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    lane_y: f32,
    color: Srgba,
) -> Entity {
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::from_length(1.8))),
            MeshMaterial3d(
                materials.add(StandardMaterial::from_color(color)),
            ),
            Transform::from_translation(
                START + Vec3::new(0.0, lane_y, 0.0),
            ),
        ))
        .id()
}

fn spawn_timeline(
    mut commands: Commands,
    mut motiongfx: ResMut<MotionGfxManager>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let compose = spawn_cube(
        &mut commands,
        &mut meshes,
        &mut materials,
        3.0,
        tailwind::LIME_300,
    );
    let conflict = spawn_cube(
        &mut commands,
        &mut meshes,
        &mut materials,
        -3.0,
        tailwind::ROSE_300,
    );

    let mut b = motiongfx.create_builder();
    let ease = ease::cubic::ease_in_out;

    // Top: chained, so no time overlap. `translation` lifts the cube,
    // then `translation::x` continues x from the composed position.
    let compose_track = [
        b.act(compose, path!(<Transform>::translation), |_| {
            START + Vec3::new(3.0, 6.0, 0.0)
        })
        .with_ease(ease)
        .play(s(2)),
        b.act(compose, path!(<Transform>::translation::x), |_| {
            START.x + 10.0
        })
        .with_ease(ease)
        .play(s(2)),
    ]
    .ord_chain();

    // Bottom: `translation::x` starts one second in, overlapping
    // `translation`. The `translation` clip is removed at compile.
    let conflict_track = [
        b.act(conflict, path!(<Transform>::translation), |_| {
            START + Vec3::new(3.0, 6.0, 0.0)
        })
        .with_ease(ease)
        .play(s(4)),
        b.act(conflict, path!(<Transform>::translation::x), |_| {
            START.x + 10.0
        })
        .with_ease(ease)
        .play(s(3)),
    ]
    .ord_flow(s(1));

    let track = [compose_track, conflict_track].ord_all().compile();
    let timeline = b.compile(track);

    commands.spawn((
        motiongfx.add_timeline(timeline),
        RealtimePlayer::new().with_playing(true),
    ));
}

fn loop_timeline(
    mut motiongfx: ResMut<MotionGfxManager>,
    q_timelines: Query<&TimelineId>,
) {
    for id in &q_timelines {
        if let Some(timeline) = motiongfx.get_timeline_mut(id)
            && timeline.is_complete()
        {
            timeline.set_target_track(0);
            timeline.set_target_time(Duration::ZERO);
        }
    }
}
