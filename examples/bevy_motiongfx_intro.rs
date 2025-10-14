use bevy::prelude::*;
use bevy_motiongfx::prelude::*;
use bevy_motiongfx::typst::{TypstCompiler, TypstCompilerPlugin};
use motiongfx_vello::bevy_vello_renderer::vello::{kurbo, peniko};

fn main() {
    App::new()
        // Bevy plugins
        .add_plugins(DefaultPlugins)
        // Custom plugins
        .add_plugins((
            MotionGfx,
            MotionGfxBevy,
            MotionGfxVello,
            TypstCompilerPlugin::new(Vec::new()),
        ))
        .add_systems(Startup, (setup, bevy_motiongfx_intro))
        .add_systems(Update, timeline_movement)
        .run();
}

/*
How I want the animation to be created

fn create_animation(
    // Maybe a macro to generate the required system inputs???
    mut commands: Commands,
    mut scenes: SceneAsset,
    mut typst_compiler: TypstCompuler
) {
    // Will spawn out a fully featured circle/rect entity with transform etc.
    let circle = circle!(100.0);
    let rect = rect!();

    let motion0 = chain! {
        translate(circle, Vec3::Y * 100.0),
        all! {
            enlarge_cricle!(circle, 10.0),
            enlarge_rect!(rect, 3.0),
        },
    }

    let motion1 = all! {
        motion0,
        alpha_circle(circle, 0.0),
        alpha_rect(rect, 0.0),
    }
}
*/

pub struct WordTrace {
    pub transform: TransformMotion,
    pub fill: FillStyleMotion,
    pub trace: VelloBezPathMotion,
}

type WordTraceVec = Vec<WordTrace>;

fn bevy_motiongfx_intro(
    mut commands: Commands,
    mut scenes: ResMut<Assets<VelloScene>>,
    mut typst_compiler: ResMut<TypstCompiler>,
) {
    let color_palette = ColorPalette::default();

    let (mut intro, intro_setup) = create_typst(
        &mut commands,
        &mut typst_compiler,
        &mut scenes,
        &color_palette,
        "#align(horizon + center)[ == Introducing]",
    );

    let intro_trace = per_letter_trace(&mut commands, &mut intro);
    let intro_move_up = per_letter_translate(&mut commands, &mut intro, Vec3::Y * 72.0);

    let (mut title, title_setup) = create_typst(
        &mut commands,
        &mut typst_compiler,
        &mut scenes,
        &color_palette,
        "#align(horizon + center)[ = Bevy MotionGfx]",
    );

    let title_trace = per_letter_trace(&mut commands, &mut title);

    let setup_seq = all!(intro_setup, title_setup);
    let sequence = chain!(setup_seq, intro_trace, all!(intro_move_up, title_trace));

    commands.spawn(SequencePlayerBundle {
        sequence,
        ..default()
    });
}

fn create_typst(
    commands: &mut Commands,
    typst_compiler: &mut ResMut<TypstCompiler>,
    scenes: &mut ResMut<Assets<VelloScene>>,
    color_palette: &ColorPalette<ColorKey>,
    content: &str,
) -> (WordTraceVec, Sequence) {
    let header = r###"
    #set page(width: 1280pt, height: 720pt, margin: 72pt)
    #set raw(theme: "themes/Monokai Pro.tmTheme")
    #set text(size: 32pt, fill: rgb("#FCFCFA00"))
    "###;

    let content = String::from(header) + content;

    let mut tree = typst_compiler
        .compile_flatten(commands, scenes, content)
        .unwrap();

    commands
        .entity(tree.root_entity)
        .insert(Transform::from_xyz(-1280.0 / 2.0, 720.0 / 2.0, 0.0));

    let mut word_trace_vec = WordTraceVec::with_capacity(tree.paths.len());
    let mut setup_seqs = Vec::with_capacity(tree.paths.len());

    for path in &mut tree.paths {
        let fill = {
            match &path.fill {
                Some(fill) => fill.clone(),
                None => {
                    let fill = FillStyle::new(
                        peniko::Fill::NonZero,
                        *color_palette.get(&ColorKey::lightest()).unwrap(),
                        kurbo::Affine::IDENTITY,
                    );

                    path.fill = Some(fill.clone());
                    fill
                }
            }
        };

        let stroke = {
            match &path.stroke {
                Some(stroke) => stroke.clone(),
                None => StrokeStyle::new(
                    1.0,
                    *color_palette.get(&ColorKey::lightest()).unwrap(),
                    fill.transform,
                ),
            }
        };

        commands
            .entity(path.entity)
            .insert(fill.clone())
            .insert(stroke.clone());

        let mut path_motion = VelloBezPathMotion::new(path.entity);
        let mut fill_motion = FillStyleMotion::new(path.entity, fill.clone());

        setup_seqs.push(all!(
            commands.play(path_motion.trace_to(0.0), 0.0),
            commands.play(fill_motion.alpha_to(0.0), 0.0),
        ));

        word_trace_vec.push(WordTrace {
            transform: TransformMotion::new(path.entity, path.transform),
            fill: fill_motion,
            trace: path_motion,
        });
    }

    (word_trace_vec, all!(&setup_seqs))
}

fn per_letter_trace(commands: &mut Commands, word_trace_vec: &mut WordTraceVec) -> Sequence {
    let motions: Vec<Sequence> = word_trace_vec
        .iter_mut()
        .map(|word_trace| {
            flow!(
                0.5,
                commands.play(word_trace.trace.trace_to(1.0), 1.0),
                commands.play(word_trace.fill.alpha_to(1.0), 1.0),
            )
        })
        .collect();

    flow!(0.1, &motions).with_ease(ease::cubic::ease_in_out)
}

fn per_letter_translate(
    commands: &mut Commands,
    word_trace_vec: &mut WordTraceVec,
    translation: Vec3,
) -> Sequence {
    let motions: Vec<Sequence> = word_trace_vec
        .iter_mut()
        .map(|word_trace| commands.play(word_trace.transform.translate_add(translation), 1.0))
        .collect();

    flow!(0.1, &motions).with_ease(ease::quart::ease_in_out)
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
