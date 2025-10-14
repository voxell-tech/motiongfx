use bevy::prelude::*;
use bevy_motiongfx::prelude::*;
use motiongfx_typst::{TypstCompiler, TypstCompilerPlugin};
use motiongfx_vello::{bevy_vello_renderer::vello::peniko, svg};

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
        .add_systems(Startup, (setup, typst_basic))
        .add_systems(Update, timeline_movement_system)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn typst_basic(
    mut commands: Commands,
    mut sequence: ResMut<Sequence>,
    mut typst_compiler: ResMut<TypstCompiler>,
    mut fragment_assets: ResMut<Assets<VelloFragment>>,
) {
    let content: String = String::from(
        r###"
        #set page(width: 1120pt, margin: 8pt)
        #set raw(theme: "themes/Monokai Pro.tmTheme")
        #set text(size: 24pt, font: "consolas", fill: rgb("#FCFCFA"))

        #show raw: set text(font: "Consolas")

        #box()[
            #text(fill: gradient.linear(rgb("#13A8C1"), rgb("#21C0AA")))[= Typst]
        ]

        $ frac(a^2, 2) $
        $ vec(1, 2, delim: "[") $
        $ mat(1, 2; 3, 4) $
        $ lim_x = op("lim", limits: #true)_x $

        ```rs
        fn main() {
            println!("Hello from Typst!");
        }
        ```
        "###,
    );

    match typst_compiler.compile_flatten(&mut commands, &mut fragment_assets, content) {
        Ok(tree) => {
            commands
                .entity(tree.root_entity)
                .insert(Transform::from_xyz(-500.0, 300.0, 0.0));

            // Motion
            let path_len: usize = tree.paths.len();

            let mut transform_motions: Vec<TransformMotion> = Vec::with_capacity(path_len);
            let mut fill_motions: Vec<Option<FillStyleMotion>> = Vec::with_capacity(path_len);
            let mut stroke_motions: Vec<Option<StrokeStyleMotion>> = Vec::with_capacity(path_len);

            for p in 0..path_len {
                let path: &svg::SvgPathBundle = &tree.paths[p];

                transform_motions.push(TransformMotion::new(path.entity, path.transform));

                if let Some(fill) = &path.fill {
                    fill_motions.push(Some(FillStyleMotion::new(path.entity, fill.clone())));
                } else {
                    fill_motions.push(None);
                }

                if let Some(stroke) = &path.stroke {
                    stroke_motions.push(Some(StrokeStyleMotion::new(path.entity, stroke.clone())));
                } else {
                    stroke_motions.push(None);
                }
            }

            // Actions
            let mut act: ActionBuilder = ActionBuilder::new(&mut commands);

            let mut setup_actions: Vec<ActionMetaGroup> = Vec::with_capacity(path_len);
            let mut animate_actions: Vec<ActionMetaGroup> = Vec::with_capacity(path_len);

            let transform_offset: Vec3 = Vec3::Y * 24.0;

            for p in 0..path_len {
                let path: &svg::SvgPathBundle = &tree.paths[p];

                if let Some(motion) = &mut fill_motions[p] {
                    setup_actions.push(act.play(motion.brush_to(Color::NONE), 0.0));
                }
                if let Some(motion) = &mut stroke_motions[p] {
                    setup_actions.push(act.play(motion.brush_to(Color::NONE), 0.0));
                }

                animate_actions.push(all(&[
                    act.play(transform_motions[p].translate_add(transform_offset), 1.0),
                    {
                        if let Some(motion) = &mut fill_motions[p] {
                            let brush: peniko::Brush = path.fill.as_ref().unwrap().brush.clone();
                            act.play(motion.brush_to(brush), 1.0)
                        } else {
                            act.sleep(1.0)
                        }
                    },
                    {
                        if let Some(motion) = &mut stroke_motions[p] {
                            let brush: peniko::Brush = path.stroke.as_ref().unwrap().brush.clone();
                            act.play(motion.brush_to(brush), 1.0)
                        } else {
                            act.sleep(1.0)
                        }
                    },
                ]));
            }

            sequence.play(
                all(&[all(&setup_actions), flow(0.1, &animate_actions)])
                    .with_ease(ease::expo::ease_in_out),
            );
        }
        Err(err) => {
            println!("{:#?}", err);
        }
    }
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
