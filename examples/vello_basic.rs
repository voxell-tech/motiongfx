use bevy::{math::DVec2, prelude::*};
use bevy_motiongfx::motiongfx_vello::bevy_vello_graphics::bevy_vello::{prelude::*, VelloPlugin};
use bevy_motiongfx::{prelude::*, MotionGfxPlugin};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, VelloPlugin::default()))
        // Custom plugins
        .add_plugins(MotionGfxPlugin)
        .add_systems(Startup, (setup, vello_basic))
        .add_systems(Update, timeline_movement)
        .run();
}

fn vello_basic(mut commands: Commands) {
    // Color palette
    let palette = ColorPalette::default();

    // Create vello graphics
    let line = (
        VelloLine::new(DVec2::new(-300.0, 0.0), DVec2::new(300.0, 0.0)),
        Stroke::default().with_brush(Brush::from_color(palette.get(ColorKey::Base8))),
        Transform::from_xyz(0.0, -100.0, 0.0),
    );
    let id = commands
        .spawn(VelloSceneBundle::default())
        .insert(line.clone())
        .id();
    let mut line = (id, line);

    let rect = (
        VelloRect::new(100.0, 100.0),
        Fill::new().with_color(palette.get(ColorKey::Blue)),
        Stroke::new(4.0).with_color(palette.get(ColorKey::Blue).lighter(0.2)),
        Transform::from_xyz(-200.0, 0.0, 0.0),
    );
    let id = commands
        .spawn(VelloSceneBundle::default())
        .insert(rect.clone())
        .id();
    let mut rect = (id, rect);

    let circle = (
        VelloCircle::new(50.0),
        Fill::new().with_color(palette.get(ColorKey::Purple)),
        Stroke::new(4.0).with_color(palette.get(ColorKey::Purple).lighter(0.2)),
        Transform::from_xyz(200.0, 0.0, 0.0),
    );
    let id = commands
        .spawn(VelloSceneBundle::default())
        .insert(circle.clone())
        .id();
    let mut circle = (id, circle);

    // Generate sequence
    let line_seq = [
        commands
            .add_motion({
                let y = line.transform().transform.translation.y;
                line.transform().to_translation_y(y - 100.0).animate(1.5)
            })
            .add_motion(
                act!(
                    (line.id(), VelloLine),
                    start = { *line.get_mut::<VelloLine>() },
                    end = line.get_mut::<VelloLine>().extend(100.0),
                )
                .animate(1.0),
            )
            .add_motion(line.stroke().to_width(10.0).animate(1.0))
            .all(),
        commands
            .add_motion({
                let y = line.transform().transform.translation.y;
                line.transform().to_translation_y(y + 100.0).animate(1.5)
            })
            .add_motion(
                act!(
                    (line.id(), VelloLine),
                    start = { *line.get_mut::<VelloLine>() },
                    end = line.get_mut::<VelloLine>().extend(-100.0),
                )
                .animate(1.0),
            )
            .add_motion(line.stroke().to_width(1.0).animate(1.0))
            .all(),
    ]
    .chain();

    let rect_seq = [
        commands
            .add_motion(
                act!(
                    (rect.id(), VelloRect),
                    start = { rect.get_mut::<VelloRect>() }.size,
                    end = rect.get_mut::<VelloRect>().size + 50.0,
                )
                .animate(1.0),
            )
            .add_motion(
                rect.transform()
                    .to_rotation(Quat::from_euler(
                        EulerRot::XYZ,
                        0.0,
                        0.0,
                        std::f32::consts::PI,
                    ))
                    .animate(1.0),
            )
            .add_motion(rect.stroke().to_width(20.0).animate(1.0))
            .all(),
        commands
            .add_motion(
                act!(
                    (rect.id(), VelloRect),
                    start = { rect.get_mut::<VelloRect>() }.size,
                    end = rect.get_mut::<VelloRect>().size - 50.0,
                )
                .animate(1.0),
            )
            .add_motion(
                rect.transform()
                    .to_rotation(Quat::from_euler(
                        EulerRot::XYZ,
                        0.0,
                        0.0,
                        std::f32::consts::TAU,
                    ))
                    .animate(1.0),
            )
            .add_motion(rect.stroke().to_width(4.0).animate(1.0))
            .all(),
    ]
    .chain();

    let cirlce_seq = [
        commands
            .add_motion(
                act!(
                    (circle.id(), VelloCircle),
                    start = { circle.get_mut::<VelloCircle>() }.radius,
                    end = circle.get_mut::<VelloCircle>().radius + 50.0,
                )
                .animate(1.0),
            )
            .add_motion(circle.stroke().to_width(20.0).animate(1.0))
            .all(),
        commands
            .add_motion(
                act!(
                    (circle.id(), VelloCircle),
                    start = { circle.get_mut::<VelloCircle>() }.radius,
                    end = circle.get_mut::<VelloCircle>().radius - 50.0,
                )
                .animate(1.0),
            )
            .add_motion(circle.stroke().to_width(4.0).animate(1.0))
            .all(),
    ]
    .chain();

    let sequence = [line_seq, rect_seq, cirlce_seq].flow(0.5);

    commands.spawn(SequencePlayerBundle {
        sequence,
        ..default()
    });
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn timeline_movement(
    mut q_timelines: Query<(&mut SequencePlayer, &mut SequenceController)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    for (mut sequence_player, mut sequence_time) in q_timelines.iter_mut() {
        if keys.pressed(KeyCode::KeyD) {
            sequence_time.target_time += time.delta_seconds();
        }

        if keys.pressed(KeyCode::KeyA) {
            sequence_time.target_time -= time.delta_seconds();
        }

        if keys.just_pressed(KeyCode::Space) {
            if keys.pressed(KeyCode::ShiftLeft) {
                sequence_player.time_scale = -1.0;
            } else {
                sequence_player.time_scale = 1.0;
            }
        }

        if keys.just_pressed(KeyCode::Escape) {
            sequence_player.time_scale = 0.0;
        }
    }
}
