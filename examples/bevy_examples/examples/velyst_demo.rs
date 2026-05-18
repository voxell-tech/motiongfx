use bevy::prelude::*;
use bevy_examples::timeline_movement;
use bevy_motiongfx::BevyMotionGfxPlugin;
use bevy_motiongfx::prelude::*;
use bevy_vello::prelude::VelloView;
use peniko_motiongfx::prelude::*;
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
            (animate_trace_fade, animate_trace)
                .in_set(VelystSet::PostLayout),
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

    let mut b = motiongfx.create_builder();

    let plot_kanva = commands
        .spawn((
            VelystFunc::new(handle.clone(), PlotFunc::default()),
            WorldScene::default().with_anchor(Vec2::splat(0.5)),
            VelystKanva::default(),
        ))
        .id();

    let eq_kanva = commands
        .spawn((
            VelystFunc::new(handle, EqFunc::default()),
            WorldScene::default(),
            VelystKanva::default(),
            Transform::from_xyz(100.0, 300.0, 0.0),
        ))
        .id();

    let plot_anim = commands
        .spawn(TraceKanva {
            kanva: Some(plot_kanva),
            group: KanvaGroup::Wrap("grid-start", "grid-end"),
            ..default()
        })
        .id();

    let eq_anim = commands
        .spawn(TraceFadeKanva {
            kanva: Some(eq_kanva),
            ..default()
        })
        .id();

    let frag = [
        b.act(plot_anim, path!(<TraceKanva>::t), |_| 1.0)
            .with_ease(ease::cubic::ease_in_out)
            .play(3.0),
        b.act(eq_anim, path!(<TraceFadeKanva>::t), |_| 1.0)
            .with_ease(ease::cubic::ease_in_out)
            .play(3.0),
    ]
    .ord_chain();

    b.add_tracks(frag.compile());

    let timeline = b.compile();
    commands.spawn((
        motiongfx.add_timeline(timeline),
        RealtimePlayer::new().with_playing(true),
    ));
}

fn animate_trace(
    q: Query<(Entity, &TraceKanva)>,
    mut kanva_q: Query<&mut VelystKanva>,
) {
    for (entity, trace) in &q {
        let kanva_entity = trace.kanva.unwrap_or(entity);
        let Ok(mut kanva) = kanva_q.get_mut(kanva_entity) else {
            continue;
        };
        if kanva.is_empty() {
            continue;
        }

        let range = match &trace.group {
            KanvaGroup::Wrap(start_name, end_name) => {
                let (Some(start_idx), Some(end_idx)) = (
                    kanva.query_group(start_name),
                    kanva.query_group(end_name),
                ) else {
                    continue;
                };
                let Some(range) = kanva
                    .get_paths_between_groups(start_idx, end_idx)
                else {
                    continue;
                };
                range
            }
            KanvaGroup::Inner(name) => {
                let Some(idx) = kanva.query_group(name) else {
                    continue;
                };
                let Some(range) = kanva.get_group_path_range(idx)
                else {
                    continue;
                };
                range
            }
            KanvaGroup::All => {
                let mut n = 0usize;
                while kanva.get_path(n).is_some() {
                    n += 1;
                }
                0..n
            }
        };

        let n = range.len();
        if n == 0 {
            continue;
        }

        let t = trace.t;
        let path_window = trace.path_window;
        let stagger = if n > 1 {
            (1.0 - path_window) / (n - 1) as f32
        } else {
            0.0
        };

        let originals = range
            .clone()
            .filter_map(|i| Some(kanva.get_path(i)?.path.clone()))
            .collect::<Vec<_>>();

        for (i, (path_idx, orig)) in
            range.zip(originals.iter()).enumerate()
        {
            let local_t = ((t - i as f32 * stagger) / path_window)
                .clamp(0.0, 1.0);
            let traced = PathTracer {
                path: orig.clone(),
                t_start: 0.0,
                t_end: local_t,
            }
            .trace();
            kanva.mod_path(path_idx).shape(traced);
        }
    }
}

fn animate_trace_fade(
    q: Query<(Entity, &TraceFadeKanva)>,
    mut kanva_q: Query<&mut VelystKanva>,
) {
    for (entity, trace_fade) in &q {
        let kanva_entity = trace_fade.kanva.unwrap_or(entity);
        let Ok(mut kanva) = kanva_q.get_mut(kanva_entity) else {
            continue;
        };
        if kanva.is_empty() {
            continue;
        }

        let range = match &trace_fade.group {
            KanvaGroup::Wrap(start_name, end_name) => {
                let (Some(start_idx), Some(end_idx)) = (
                    kanva.query_group(start_name),
                    kanva.query_group(end_name),
                ) else {
                    continue;
                };
                let Some(range) = kanva
                    .get_paths_between_groups(start_idx, end_idx)
                else {
                    continue;
                };
                range
            }
            KanvaGroup::Inner(name) => {
                let Some(idx) = kanva.query_group(name) else {
                    continue;
                };
                let Some(range) = kanva.get_group_path_range(idx)
                else {
                    continue;
                };
                range
            }
            KanvaGroup::All => {
                let mut n = 0usize;
                while kanva.get_path(n).is_some() {
                    n += 1;
                }
                0..n
            }
        };

        let n = range.len();
        if n == 0 {
            continue;
        }

        let t = trace_fade.t;
        let path_window = trace_fade.path_window;
        let trace_ratio = trace_fade.trace_ratio;
        let stagger = if n > 1 {
            (1.0 - path_window) / (n - 1) as f32
        } else {
            0.0
        };

        let originals = range
            .clone()
            .filter_map(|i| {
                let path = kanva.get_path(i)?;
                let fill = path
                    .fill
                    .and_then(|fi| kanva.get_fill(fi))
                    .cloned();
                Some((path.path.clone(), fill))
            })
            .collect::<Vec<_>>();

        for (i, (path_idx, (orig, fill))) in
            range.zip(originals.iter()).enumerate()
        {
            let local_t = ((t - i as f32 * stagger) / path_window)
                .clamp(0.0, 1.0);

            let trace_t = (local_t / trace_ratio).min(1.0);
            let fade_t = ((local_t - trace_ratio)
                / (1.0 - trace_ratio))
                .clamp(0.0, 1.0);

            let faded_fill = fill.clone().map(|mut f| {
                f.brush = f.brush.with_alpha(fade_t);
                f
            });
            let traced = PathTracer {
                path: orig.clone(),
                t_start: 0.0,
                t_end: trace_t,
            }
            .trace();
            kanva.mod_path(path_idx).shape(traced).fill(faded_fill);
        }
    }
}

enum KanvaGroup {
    /// Explicit start and end group markers.
    Wrap(&'static str, &'static str),
    /// Single group name.
    Inner(&'static str),
    /// All paths in the kanva.
    All,
}

#[derive(Component)]
struct TraceKanva {
    t: f32,
    path_window: f32,
    kanva: Option<Entity>,
    group: KanvaGroup,
}

impl Default for TraceKanva {
    fn default() -> Self {
        Self {
            t: 0.0,
            path_window: 0.3,
            kanva: None,
            group: KanvaGroup::All,
        }
    }
}

#[derive(Component)]
struct TraceFadeKanva {
    t: f32,
    path_window: f32,
    trace_ratio: f32,
    kanva: Option<Entity>,
    group: KanvaGroup,
}

impl Default for TraceFadeKanva {
    fn default() -> Self {
        Self {
            t: 0.0,
            path_window: 0.5,
            trace_ratio: 0.6,
            kanva: None,
            group: KanvaGroup::All,
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
