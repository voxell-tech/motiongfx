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
        .register_typst_func::<EquationFunc>()
        .register_typst_func::<PlotFunc>()
        .add_systems(Startup, setup)
        .add_systems(Update, timeline_movement)
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
            VelystFunc::new(handle, EquationFunc::default()),
            WorldScene::default(),
            VelystKanva::default(),
            Transform::from_xyz(100.0, 300.0, 0.0),
            TraceFadeKanva::default(),
            KanvaGroup::default(),
        ))
        .id();

    let grid = commands
        .spawn((
            TraceKanva::default(),
            KanvaGroup::wrap(plot, "grid-start", "grid-end"),
        ))
        .id();

    let circle = commands
        .spawn((
            TraceFadeKanva::default(),
            KanvaGroup::wrap(plot, "circle-start", "circle-end"),
        ))
        .id();

    let frag = [
        [
            b.act(grid, path!(<TraceKanva>::t), |_| 1.0)
                .with_ease(ease::cubic::ease_in_out)
                .play(3.0),
            b.act(circle, path!(<TraceFadeKanva>::t), |_| 1.0)
                .with_ease(ease::cubic::ease_in_out)
                .play(3.0),
        ]
        .ord_flow(1.0),
        b.act(equation, path!(<TraceFadeKanva>::t), |_| 1.0)
            .with_ease(ease::cubic::ease_in_out)
            .play(2.0), // .delay(1.0),
    ]
    .ord_chain();

    b.add_tracks(frag.compile());

    let timeline = b.compile();
    commands.spawn((
        motiongfx.add_timeline(timeline),
        RealtimePlayer::new().with_playing(true),
    ));
}

typst_func!(
    "equation",
    #[derive(Default)]
    struct EquationFunc {},
);

typst_func!(
    "plot",
    #[derive(Default)]
    struct PlotFunc {},
    positional_args {
        circle_x: f64,
        circle_y: f64
    }
);
