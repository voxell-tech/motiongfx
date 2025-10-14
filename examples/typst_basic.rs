fn main() {}
// use bevy::prelude::*;
// use bevy_motiongfx::prelude::*;
// use bevy_motiongfx::typst::{TypstCompiler, TypstCompilerPlugin};

// fn main() {
//     App::new()
//         // Bevy plugins
//         .add_plugins(DefaultPlugins)
//         // Custom plugins
//         .add_plugins((
//             MotionGfxPlugin,
//             MotionGfxVelloPlugin,
//             TypstCompilerPlugin::new(Vec::new()),
//         ))
//         .add_systems(Startup, (setup, typst_basic))
//         .add_systems(Update, timeline_movement)
//         .run();
// }

// fn typst_basic(
//     mut commands: Commands,
//     mut typst_compiler: ResMut<TypstCompiler>,
//     mut scenes: ResMut<Assets<VelloScene>>,
// ) {
//     let content = String::from(
//         r###"
//         #set page(width: 1120pt, margin: 8pt)
//         #set raw(theme: "themes/Monokai Pro.tmTheme")
//         #set text(size: 24pt, fill: rgb("#FCFCFA"))

//         #box()[
//             #text(fill: gradient.linear(rgb("#13A8C1"), rgb("#21C0AA")))[= Typst]
//         ]

//         $ frac(a^2, 2) $
//         $ vec(1, 2, delim: "[") $
//         $ mat(1, 2; 3, 4) $
//         $ lim_x = op("lim", limits: #true)_x $

//         ```rs
//         fn main() {
//             println!("Hello from Typst!");
//         }
//         ```
//         "###,
//     );

//     match typst_compiler.compile_flatten(&mut commands, &mut scenes, content) {
//         Ok(tree) => {
//             commands
//                 .entity(tree.root_entity)
//                 .insert(Transform::from_xyz(-500.0, 300.0, 0.0));

//             // Motion
//             let path_len = tree.paths.len();

//             let path_ids = Vec::with_capacity(path_len);
//             let mut transforms = Vec::with_capacity(path_len);
//             let mut fill_motions = Vec::with_capacity(path_len);
//             let mut stroke_motions = Vec::with_capacity(path_len);

//             // tree.paths.iter().map();

//             for p in 0..path_len {
//                 let path = &tree.paths[p];

//                 path_ids.push(path.entity);
//                 transforms.push(path.transform);

//                 if let Some(fill) = &path.fill {
//                     fill_motions.push(Some(FillStyleMotion::new(path.entity, fill.clone())));
//                 } else {
//                     fill_motions.push(None);
//                 }

//                 if let Some(stroke) = &path.stroke {
//                     stroke_motions.push(Some(StrokeStyleMotion::new(path.entity, stroke.clone())));
//                 } else {
//                     stroke_motions.push(None);
//                 }
//             }

//             // Animations
//             let mut setup_seqs = Vec::with_capacity(path_len);
//             let mut animate_seqs = Vec::with_capacity(path_len);

//             let transform_offset = Vec3::Y * 24.0;

//             for p in 0..path_len {
//                 let path = &tree.paths[p];

//                 if let Some(motion) = &mut fill_motions[p] {
//                     setup_seqs.push(commands.play(motion.brush_to(Color::NONE), 0.0));
//                 }
//                 if let Some(motion) = &mut stroke_motions[p] {
//                     setup_seqs.push(commands.play(motion.brush_to(Color::NONE), 0.0));
//                 }

//                 animate_seqs.push(all(&[
//                     commands.play(transforms[p].translate_add(transform_offset), 1.0),
//                     {
//                         if let Some(motion) = &mut fill_motions[p] {
//                             let brush = path.fill.as_ref().unwrap().brush.clone();
//                             commands.play(motion.brush_to(brush), 1.0)
//                         } else {
//                             commands.sleep(1.0)
//                         }
//                     },
//                     {
//                         if let Some(motion) = &mut stroke_motions[p] {
//                             let brush = path.stroke.as_ref().unwrap().brush.clone();
//                             commands.play(motion.brush_to(brush), 1.0)
//                         } else {
//                             commands.sleep(1.0)
//                         }
//                     },
//                 ]));
//             }

//             let sequence = all(&[all(&setup_seqs), flow(0.1, &animate_seqs)])
//                 .with_ease(ease::expo::ease_in_out);

//             commands.spawn(SequencePlayerBundle {
//                 sequence,
//                 ..default()
//             });
//         }
//         Err(err) => {
//             println!("{:#?}", err);
//         }
//     }
// }

// fn setup(mut commands: Commands) {
//     commands.spawn(Camera2dBundle::default());
// }

// fn timeline_movement(
//     mut q_timelines: Query<(&mut SequencePlayer, &mut SequenceController)>,
//     keys: Res<ButtonInput<KeyCode>>,
//     time: Res<Time>,
// ) {
//     for (mut sequence_player, mut sequence_time) in q_timelines.iter_mut() {
//         if keys.pressed(KeyCode::KeyD) {
//             sequence_time.target_time += time.delta_seconds();
//         }

//         if keys.pressed(KeyCode::KeyA) {
//             sequence_time.target_time -= time.delta_seconds();
//         }

//         if keys.just_pressed(KeyCode::Space) {
//             if keys.pressed(KeyCode::ShiftLeft) {
//                 sequence_player.time_scale = -1.0;
//             } else {
//                 sequence_player.time_scale = 1.0;
//             }
//         }

//         if keys.just_pressed(KeyCode::Escape) {
//             sequence_player.time_scale = 0.0;
//         }
//     }
// }
