use bevy::prelude::*;
use bevy_examples::timeline_movement;
use bevy_motiongfx::BevyMotionGfxPlugin;
use bevy_motiongfx::prelude::*;
use bevy_vello::prelude::VelloView;
use velyst::prelude::*;
use velyst_motiongfx::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(AssetPlugin {
                file_path: concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets"
                )
                .into(),
                ..default()
            }),
            bevy_vello::VelloPlugin::default(),
            velyst::VelystPlugin,
            BevyMotionGfxPlugin,
            VelystMotionGfxPlugin,
        ))
        .register_typst_func::<PlotFunc>()
        .add_systems(Startup, setup)
        .add_systems(Update, (timeline_movement, perf_metrics))
        .run();
}

fn setup(
    mut commands: Commands,
    mut motiongfx: ResMut<MotionGfxManager>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
        VelloView,
    ));

    let handle = asset_server.load("typst/velyst_demo.typ");

    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        PerfMetrics,
    ));

    let mut b = motiongfx.create_builder();

    let plot = commands
        .spawn((
            VelystFunc::new(handle.clone(), PlotFunc::default()),
            WorldScene::default().with_anchor(Vec2::splat(0.5)),
            VelystKanva::default(),
        ))
        .id();

    let equation = commands
        .spawn((
            KanvaGroup::wrap("coord-start", "coord-end")
                .with_target(plot),
            TraceFadeKanva::default(),
        ))
        .id();

    let grid = commands
        .spawn((
            KanvaGroup::wrap("grid-start", "grid-end")
                .with_target(plot),
            TraceKanva::default(),
        ))
        .id();

    let circle = commands
        .spawn((
            KanvaGroup::wrap("circle-start", "circle-end")
                .with_target(plot),
            TraceFadeKanva::default(),
        ))
        .id();

    let frag = [
        b.act(grid, path!(<TraceKanva>::t), |_| 1.0)
            .with_ease(ease::cubic::ease_in_out)
            .play(2.0),
        b.act(circle, path!(<TraceFadeKanva>::t), |_| 1.0)
            .with_ease(ease::cubic::ease_in_out)
            .play(1.0),
        b.act(equation, path!(<TraceFadeKanva>::t), |_| 1.0)
            .with_ease(ease::cubic::ease_in_out)
            .play(2.0),
        b.act(plot, path!(<VPlotFunc>::data::circle_x), |_| 3.0)
            .with_ease(ease::cubic::ease_in_out)
            .play(2.0),
        b.act(plot, path!(<VPlotFunc>::data::circle_y), |_| 4.0)
            .with_ease(ease::cubic::ease_in_out)
            .play(2.0),
    ]
    .ord_chain();

    b.add_tracks(frag.compile());

    let timeline = b.compile();
    commands.spawn((
        motiongfx.add_timeline(timeline),
        RealtimePlayer::new().with_playing(true),
    ));
}

#[derive(Component)]
struct PerfMetrics;

fn perf_metrics(
    time: Res<Time>,
    mut q: Query<&mut Text, With<PerfMetrics>>,
) {
    let Ok(mut text) = q.single_mut() else { return };
    let fps = (1.0 / time.delta_secs_f64() * 100.0).round() / 100.0;
    let elapsed = (time.elapsed_secs_f64() * 100.0).round() / 100.0;
    **text = format!("FPS: {fps}\nElapsed: {elapsed}");
}

type VPlotFunc = VelystFunc<PlotFunc>;

typst_func!(
    "plot",
    #[derive(Default)]
    struct PlotFunc {},
    positional_args {
        circle_x: f64,
        circle_y: f64
    }
);
