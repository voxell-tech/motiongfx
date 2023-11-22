use bevy::{
    math::{DVec2, DVec4},
    prelude::*,
};
use bevy_motiongfx::prelude::*;
use motiongfx_vello::{
    bevy_vello_renderer::prelude::*,
    fill_style::FillStyle,
    stroke_style::StrokeStyle,
    vello_vector::rect::{VelloRect, VelloRectBundle, VelloRectBundleMotion},
};

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
    const RECT_COUNT: usize = 14;
    const RECT_SIZE: f32 = 40.0;
    const SPACING: f32 = 5.0;

    // Color palette
    let palette: ColorPalette<ColorKey> = ColorPalette::default();

    let mut rect_motions: Vec<VelloRectBundleMotion> = Vec::with_capacity(RECT_COUNT);
    let mut transform_motions: Vec<TransformMotion> = Vec::with_capacity(RECT_COUNT);

    let start_y: f32 = (RECT_COUNT as f32) * 0.5 * (RECT_SIZE + SPACING);

    for r in 0..RECT_COUNT {
        let transform: Transform = Transform::from_translation(Vec3::new(
            -500.0,
            start_y - (r as f32) * (RECT_SIZE + SPACING),
            0.0,
        ));

        let rect_bundle: VelloRectBundle = VelloRectBundle {
            rect: VelloRect::anchor_center(DVec2::new(0.0, 0.0), DVec4::splat(10.0)),
            fragment_bundle: VelloFragmentBundle {
                fragment: fragments.add(VelloFragment::default()),
                transform: TransformBundle::from_transform(transform),
                ..default()
            },
            fill: FillStyle::from_brush(*palette.get_or_default(&ColorKey::Base4)),
            stroke: StrokeStyle::from_brush(*palette.get_or_default(&ColorKey::Base6)),
            ..default()
        };

        let fragment_id: Entity = commands.spawn(rect_bundle.clone()).id();

        rect_motions.push(VelloRectBundleMotion::new(fragment_id, rect_bundle));
        transform_motions.push(TransformMotion::new(fragment_id, transform));
    }

    // ACTIONS
    let mut act: ActionBuilder = ActionBuilder::new(&mut commands);

    let mut inflate_actions: Vec<ActionMetaGroup> = Vec::with_capacity(RECT_COUNT);
    let mut expand_right_actions: Vec<ActionMetaGroup> = Vec::with_capacity(RECT_COUNT);
    let mut expand_left_actions: Vec<ActionMetaGroup> = Vec::with_capacity(RECT_COUNT);
    let mut transform_actions: Vec<ActionMetaGroup> = Vec::with_capacity(RECT_COUNT);
    let mut fill_actions: Vec<ActionMetaGroup> = Vec::with_capacity(RECT_COUNT);

    for r in 0..RECT_COUNT {
        let expansion: f64 = 900.0 * (r as f64) / (RECT_COUNT as f64) + 100.0;

        inflate_actions.push(
            act.play(
                rect_motions[r].rect.inflate(Vec2::splat(RECT_SIZE * 0.5)),
                1.0,
            )
            .with_ease(ease::expo::ease_in_out),
        );
        expand_right_actions.push(
            act.play(rect_motions[r].rect.expand_right(expansion), 1.0)
                .with_ease(ease::expo::ease_in_out),
        );
        expand_left_actions.push(
            act.play(rect_motions[r].rect.expand_left(-expansion), 1.0)
                .with_ease(ease::expo::ease_in_out),
        );

        let mut translation: Vec3 = transform_motions[r].get_transform().translation;
        translation.y = 0.0;
        transform_actions.push(
            act.play(transform_motions[r].translate_to(translation), 1.0)
                .with_ease(ease::back::ease_in_out),
        );

        let color: Color = Color::lerp(
            palette.get_or_default(&ColorKey::Purple),
            palette.get_or_default(&ColorKey::Blue),
            (r as f32) / (RECT_COUNT as f32),
        );

        fill_actions.push(all(&[
            act.play(rect_motions[r].fill.brush_to(color), 1.0),
            act.play(rect_motions[r].stroke.brush_to(color * 1.2), 1.0),
            act.play(rect_motions[r].stroke.style_to(5.0), 1.0),
        ]));
    }

    sequence.play(flow(
        1.0,
        &[
            flow(0.1, &inflate_actions),
            flow(0.1, &expand_right_actions),
            flow(0.1, &expand_left_actions),
            all(&[flow(0.1, &transform_actions), flow(0.1, &fill_actions)]),
        ],
    ));
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
