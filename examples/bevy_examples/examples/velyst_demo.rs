use bevy::prelude::*;
use bevy_examples::timeline_movement;
use bevy_motiongfx::BevyMotionGfxPlugin;
use bevy_motiongfx::prelude::*;
use bevy_vello::prelude::VelloView;
use bevy_vello::vello::kurbo::BezPath;
use peniko_motiongfx::prelude::*;
use velyst::kanva::KanvaFill;
use velyst::prelude::*;

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
        ))
        .register_typst_func::<EqFunc>()
        .register_typst_func::<PlotFunc>()
        .add_systems(Startup, setup)
        .add_systems(Update, timeline_movement)
        .add_systems(
            PostUpdate,
            animate_eq.in_set(VelystSet::PostLayout),
        )
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
        VelystFunc::new(handle.clone(), PlotFunc::default()),
        WorldScene::default(),
        VelystKanva::default(),
    ));

    let mut b = motiongfx.create_builder();
    let entity = commands
        .spawn((
            VelystFunc::new(handle, EqFunc::default()),
            WorldScene::default(),
            VelystKanva::default(),
            TraceFadeKanva::default(),
            Transform::from_xyz(100.0, 300.0, 0.0),
        ))
        .id();

    let frag = b
        .act(entity, path!(<TraceFadeKanva>::t), |_| 1.0)
        .with_ease(ease::cubic::ease_in_out)
        .play(3.0);
    b.add_tracks(frag.compile());

    let timeline = b.compile();
    commands.spawn((
        motiongfx.add_timeline(timeline),
        RealtimePlayer::new().with_playing(true),
    ));
}

fn animate_eq(mut q: Query<(&TraceFadeKanva, &mut VelystKanva)>) {
    let Ok((trace_fade, mut kanva)) = q.single_mut() else {
        return;
    };
    if kanva.is_empty() {
        return;
    }

    let t = trace_fade.t;
    let path_window = trace_fade.path_window;
    let trace_ratio = trace_fade.trace_ratio;

    let n = {
        let mut i = 0usize;
        while kanva.get_path(i).is_some() {
            i += 1;
        }
        i
    };
    if n == 0 {
        return;
    }

    let stagger = if n > 1 {
        (1.0 - path_window) / (n - 1) as f32
    } else {
        0.0
    };

    // Collect originals first; mod_path needs exclusive access.
    let originals: Vec<(BezPath, Option<KanvaFill>)> = (0..n)
        .filter_map(|i| {
            let path = kanva.get_path(i)?;
            let fill =
                path.fill.and_then(|fi| kanva.get_fill(fi)).cloned();
            Some((path.path.clone(), fill))
        })
        .collect();

    for (i, (orig, fill)) in originals.into_iter().enumerate() {
        let local_t =
            ((t - i as f32 * stagger) / path_window).clamp(0.0, 1.0);

        let trace_t = (local_t / trace_ratio).min(1.0);
        let fade_t = ((local_t - trace_ratio) / (1.0 - trace_ratio))
            .clamp(0.0, 1.0);

        let faded_fill = fill.map(|mut f| {
            f.brush = f.brush.with_alpha(fade_t);
            f
        });
        let traced = PathTracer {
            path: orig,
            t_start: 0.0,
            t_end: trace_t,
        }
        .trace();
        kanva.mod_path(i).shape(traced).fill(faded_fill);
    }
}

#[derive(Component)]
struct TraceFadeKanva {
    t: f32,
    /// Fraction of total time each path's window occupies.
    path_window: f32,
    /// Fraction of each window spent tracing; remainder is fade-in.
    trace_ratio: f32,
}

impl Default for TraceFadeKanva {
    fn default() -> Self {
        Self {
            t: 0.0,
            path_window: 0.5,
            trace_ratio: 0.6,
        }
    }
}

typst_func!(
    "equation",
    #[derive(Default)]
    struct EqFunc {},
    positional_args {},
);

typst_func!(
    "plot",
    #[derive(Default)]
    struct PlotFunc {},
    positional_args {},
);
