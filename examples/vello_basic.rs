use bevy::prelude::*;
use bevy_motiongfx::prelude::*;
use motiongfx_vello::bevy_vello_renderer::prelude::*;
use motiongfx_vello::bevy_vello_renderer::vello::{kurbo, peniko, SceneBuilder, SceneFragment};

fn main() {
    App::new()
        // Bevy plugins
        .add_plugins(DefaultPlugins)
        // Custom plugins
        .add_plugins((MotionGfx, MotionGfxBevy, MotionGfxVello))
        .add_systems(Startup, (setup, vello_basic))
        .add_systems(Update, timeline_movement_system)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn vello_basic(
    mut commands: Commands,
    mut fragments: ResMut<Assets<VelloFragment>>,
    mut sequence: ResMut<Sequence>,
) {
    // Create scene fragment
    let mut fragment: SceneFragment = SceneFragment::new();
    let mut sb: SceneBuilder = SceneBuilder::for_fragment(&mut fragment);

    sb.fill(
        peniko::Fill::NonZero,
        kurbo::Affine::default(),
        &peniko::Color::WHITE_SMOKE,
        None,
        &kurbo::RoundedRect::from_rect(
            kurbo::Rect {
                x0: -50.0,
                y0: -50.0,
                x1: 50.0,
                y1: 50.0,
            },
            10.0,
        ),
    );

    let fragment_bundle: VelloFragmentBundle = VelloFragmentBundle {
        fragment: fragments.add(VelloFragment {
            fragment: fragment.into(),
        }),
        transform: TransformBundle {
            local: Transform::from_xyz(-500.0, 0.0, 0.0),
            ..default()
        },
        ..default()
    };

    let fragment_id: Entity = commands.spawn(fragment_bundle.clone()).id();

    // States
    let mut fragment_translation: Translation =
        Translation::from_transform(fragment_id, &fragment_bundle.transform.local);
    let mut fragment_rotation: Rotation =
        Rotation::from_transform(fragment_id, &fragment_bundle.transform.local);

    let mut act: ActionBuilder = ActionBuilder::new(&mut commands);

    let action_grp: ActionMetaGroup = all(&[
        act.play(fragment_translation.translate(Vec3::X * 1000.0), 1.0),
        act.play(
            fragment_rotation.rotate_to(Quat::from_rotation_z(std::f32::consts::PI)),
            1.0,
        ),
    ])
    .with_ease(ease::expo::ease_in_out);

    sequence.play(action_grp);
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
