use core::f32::consts::FRAC_PI_2;

use bevy::color::palettes;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy_motiongfx::BevyMotionGfxPlugin;
use bevy_motiongfx::manager::TimelineComplete;
use bevy_motiongfx::prelude::*;

use crate::pipelines_ready::*;

/// Time to wait after animation before closing app.
const EXIT_TIME: f32 = 0.5;
/// Directory where recorded frames will be saved.
const SAVE_DIR: &str = "frames/";

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            BevyMotionGfxPlugin,
            PipelinesReadyPlugin,
        ))
        .add_systems(
            Startup,
            (setup, spawn_timeline, setup_save_path),
        )
        .add_systems(OnEnter(PipelineState::Ready), start_recording)
        .add_systems(
            Update,
            screenshot.run_if(in_state(PipelineState::Ready)),
        )
        .add_systems(Update, check_final_frame)
        .insert_resource(ExitDelayTimer(Timer::from_seconds(
            EXIT_TIME,
            TimerMode::Once,
        )))
        .run();
}

fn screenshot(
    mut commands: Commands,
    q_player: Query<&FixedRatePlayer, Without<TimelineComplete>>,
) {
    let Ok(player) = q_player.single() else {
        return;
    };

    if !player.is_playing {
        return;
    }

    commands.spawn(Screenshot::primary_window()).observe(
        save_to_disk(format!(
            "{}frame_{:05}.png",
            SAVE_DIR, player.curr_frame
        )),
    );
}

fn start_recording(mut q_player: Query<&mut FixedRatePlayer>) {
    let Ok(mut player) = q_player.single_mut() else {
        return;
    };

    player.set_playing(true);
}
#[derive(Resource)]
struct ExitDelayTimer(Timer);

// Quit on last frame captured.
fn check_final_frame(
    incomplete_players: Query<
        &FixedRatePlayer,
        Without<TimelineComplete>,
    >,
    mut exit_timer: ResMut<ExitDelayTimer>,
    time: Res<Time>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if incomplete_players.is_empty()
        && exit_timer.0.tick(time.delta()).is_finished()
    {
        app_exit.write(AppExit::Success);
    }
}

fn spawn_timeline(
    mut commands: Commands,
    mut motiongfx: ResMut<MotionGfxManager>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn cube.
    let cube_id = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(materials.add(
                StandardMaterial::from_color(
                    palettes::tailwind::LIME_200,
                ),
            )),
            Transform::from_xyz(-5.0, 0.0, 0.0),
        ))
        .id();

    // Build the timeline.
    let mut b = motiongfx.create_builder();

    let track = b
        .act_builder(cube_id, path!(<Transform>::translation), |x| {
            x + Vec3::ZERO.with_x(10.0).with_z(1.0)
        })
        .with_interp(|start, end, t| arc_lerp_3d(*start, *end, t))
        .play(s(1))
        .compile();

    b.add_tracks(track);

    let timeline = b.compile();
    commands.spawn((
        motiongfx.add_timeline(timeline),
        FixedRatePlayer::new(144),
    ));
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
        Camera3d::default(),
        // Top down view.
        Transform::from_xyz(0.0, 18.0, 0.0)
            .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn setup_save_path() {
    match std::fs::exists(SAVE_DIR) {
        Ok(true) => (),
        Ok(false) => std::fs::create_dir(SAVE_DIR).expect(
            "Should have been able to create frame directory",
        ),
        Err(_) => panic!("could not verify 'frames/' exists"),
    };
}

// TODO: Optimize this.
pub fn arc_lerp_3d(start: Vec3, end: Vec3, t: f32) -> Vec3 {
    let center = (start + end) * 0.5;

    let start_dir = Dir3::new(start - center);
    let end_dir = Dir3::new(end - center);

    let (Ok(start_dir), Ok(end_dir)) = (start_dir, end_dir) else {
        // Revert to linear interpolation.
        return start.lerp(end, t);
    };

    let target_dir = start_dir.slerp(end_dir, t);

    center + target_dir.as_vec3() * (center - start).length()
}

mod pipelines_ready {
    use bevy::{
        prelude::*,
        render::{render_resource::*, *},
    };

    #[derive(
        States,
        Default,
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
    )]
    pub enum PipelineState {
        #[default]
        Loading,
        Ready,
    }

    pub struct PipelinesReadyPlugin;
    impl Plugin for PipelinesReadyPlugin {
        fn build(&self, app: &mut App) {
            app.init_state::<PipelineState>();

            // In order to gain access to the pipelines status, we have to
            // go into the `RenderApp`, grab the resource from the main App
            // and then update the pipelines status from there.
            // Writing between these Apps can only be done through the
            // `ExtractSchedule`.
            app.sub_app_mut(RenderApp)
                .add_systems(ExtractSchedule, update_pipelines_ready);
        }
    }

    fn update_pipelines_ready(
        mut main_world: ResMut<MainWorld>,
        pipelines: Res<PipelineCache>,
    ) {
        let curr_state =
            main_world.resource::<State<PipelineState>>();
        if *curr_state.get() == PipelineState::Ready {
            return;
        }

        let mut state =
            main_world.resource_mut::<NextState<PipelineState>>();

        // If there are pipelines cerated and all of them are already
        // initialized.
        if pipelines.pipelines().count() > 0
            && pipelines.waiting_pipelines().count() == 0
        {
            state.set(PipelineState::Ready);
        }
    }
}
