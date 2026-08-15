//! Demonstrates the [`MoxiePlugin`] timeline editor.
//!
//! A row of cubes runs through four phases - pop in together, rotate
//! in staggered pairs, race each other, then settle - deliberately
//! mixing every [`Combinator`] and nesting them inside one another, so
//! the timeline panel has a real tree to show. The editor docks a
//! timeline panel at the bottom of the window: use the play/pause
//! button to control playback and drag on the timeline to scrub.

use bevy::asset::uuid::Uuid;
use bevy::color::palettes;
use bevy::prelude::*;
use bevy_motiongfx::BevyMotionGfxPlugin;
use bevy_motiongfx::prelude::*;
use bevy_motiongfx::scene::asset::MotionGfxScene;
use bevy_motiongfx::scene::backend::{
    AnimEase, AnimInterp, AnimOp, Backend, field_ref,
};
use bevy_motiongfx::scene::id::SceneUid;
use bevy_motiongfx::scene::value_pool::ValuePool;
use moxie::MoxiePlugin;
// Aliased: `Node` and `Scene` both also name `bevy`/`bevy_ui` types
// pulled in above.
use motiongfx_scene::block::{
    ActionCmd, Block, Combinator, Node as AnimNode,
};
use motiongfx_scene::refs::FieldRef;
use motiongfx_scene::scene::{
    FieldSeed, Scene as AnimScene, Stage, Subject,
};
use motiongfx_scene::value::ValueColumn;

const CUBE_COUNT: usize = 6;

fn main() {
    App::new()
        .add_plugins((
            // `../assets`: the editor crates share one asset folder
            // (`editor/assets`) rather than each carrying its own.
            DefaultPlugins.set(AssetPlugin {
                file_path: "../assets".into(),
                ..default()
            }),
            BevyMotionGfxPlugin,
            MoxiePlugin,
        ))
        .add_systems(Startup, (setup, spawn_timeline))
        .run();
}

/// One cube: its subject id and spawn `x`, so later phases can move it
/// relative to where it started.
struct Cube {
    subject: SceneUid,
    x: f32,
}

/// The initial value of one animated field, pooled alongside the
/// action targets.
fn seed<T>(
    values: &mut ValuePool,
    field: FieldRef,
    value: T,
) -> FieldSeed<Backend>
where
    ValuePool: ValueColumn<Uuid, T>,
{
    FieldSeed {
        field,
        value: values.insert(value),
    }
}

/// Sets `subject`'s scale to `target`, eased.
fn scale_to(
    subject: SceneUid,
    values: &mut ValuePool,
    target: Vec3,
    duration: core::time::Duration,
) -> AnimNode<Backend> {
    AnimNode::action(ActionCmd {
        subject,
        field: field_ref(path!(<Transform>::scale)),
        op: AnimOp::To,
        value: values.insert(target),
        duration,
        ease: Some(AnimEase::CubicEaseInOut),
        interp: Some(AnimInterp::Linear),
    })
}

/// Sets `subject`'s rotation to `target`, eased.
fn rotate_to(
    subject: SceneUid,
    values: &mut ValuePool,
    target: Quat,
    duration: core::time::Duration,
) -> AnimNode<Backend> {
    AnimNode::action(ActionCmd {
        subject,
        field: field_ref(path!(<Transform>::rotation)),
        op: AnimOp::To,
        value: values.insert(target),
        duration,
        ease: Some(AnimEase::CubicEaseInOut),
        interp: Some(AnimInterp::Linear),
    })
}

/// Sets `subject`'s translation to `target`, eased.
fn move_to(
    subject: SceneUid,
    values: &mut ValuePool,
    target: Vec3,
    duration: core::time::Duration,
) -> AnimNode<Backend> {
    AnimNode::action(ActionCmd {
        subject,
        field: field_ref(path!(<Transform>::translation)),
        op: AnimOp::To,
        value: values.insert(target),
        duration,
        ease: Some(AnimEase::CubicEaseInOut),
        interp: Some(AnimInterp::Linear),
    })
}

/// Spawns a row of cubes and an [`EditorScene`] animating them through
/// four phases, deliberately mixing and nesting every [`Combinator`]:
///
/// 1. **Scale in** (`All`) - every cube pops in at once.
/// 2. **Paired rotate** (`Flow` of `All` pairs) - cubes rotate two at a
///    time, each pair staggered after the last.
/// 3. **Race** (`Any` of an action and a `Chain`) - two cubes chase the
///    same finish line; the phase ends the instant the faster one
///    does, leaving the slower one mid-flight.
/// 4. **Finale** (`Chain` of `All` then `Flow`) - every cube settles
///    its scale together, then unwinds its rotation one at a time.
///
/// Built as a [`Scene`] rather than through `motiongfx`'s imperative
/// track builder - the scene is what the editor edits, saves and
/// reloads; the compiled `Timeline` [`moxie::scene::recompile_dirty_scene`]
/// produces from it is just a disposable, derived view.
fn spawn_timeline(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cuboid::default());
    let mut values = ValuePool::default();

    let cubes: Vec<Cube> = (0..CUBE_COUNT)
        .map(|i| {
            let x = (i as f32) - (CUBE_COUNT as f32 - 1.0) * 0.5;
            let material =
                materials.add(StandardMaterial::from_color(
                    palettes::tailwind::SKY_400,
                ));
            let uid = EntityUid::new();
            commands.spawn((
                uid,
                Name::new(format!("Cube {i}")),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(x * 1.5, 0.0, 0.0)
                    .with_scale(Vec3::ZERO),
            ));
            Cube {
                subject: SceneUid::Entity(uid),
                x: x * 1.5,
            }
        })
        .collect();

    // Phase 1 - every cube pops in together.
    let scale_in = Block {
        combinator: Combinator::All,
        children: cubes
            .iter()
            .map(|cube| {
                scale_to(cube.subject, &mut values, Vec3::ONE, cs(40))
            })
            .collect(),
    };

    // Phase 2 - cubes rotate two at a time (an `All` pair), each pair
    // staggered after the last (a `Flow` of those pairs).
    let paired_rotate = Block {
        combinator: Combinator::Flow(cs(25)),
        children: cubes
            .chunks(2)
            .map(|pair| {
                AnimNode::block(Block {
                    combinator: Combinator::All,
                    children: pair
                        .iter()
                        .map(|cube| {
                            rotate_to(
                                cube.subject,
                                &mut values,
                                Quat::from_rotation_y(
                                    core::f32::consts::PI,
                                ),
                                s(1),
                            )
                        })
                        .collect(),
                })
            })
            .collect(),
    };

    // Phase 3 - two cubes race for the same finish line: a lone slow
    // hop against a `Chain` of two quick hops. `Any` ends the instant
    // the faster one does, so the slow hop is left unfinished.
    let race = Block {
        combinator: Combinator::Any,
        children: vec![
            move_to(
                cubes[0].subject,
                &mut values,
                Vec3::new(cubes[0].x, 1.5, 0.0),
                s(2),
            ),
            AnimNode::block(Block {
                combinator: Combinator::Chain,
                children: vec![
                    move_to(
                        cubes[1].subject,
                        &mut values,
                        Vec3::new(cubes[1].x, 1.0, 0.0),
                        cs(50),
                    ),
                    move_to(
                        cubes[1].subject,
                        &mut values,
                        Vec3::new(cubes[1].x, 0.0, 0.0),
                        cs(50),
                    ),
                ],
            }),
        ],
    };

    // Phase 4 - settle every cube's scale together (`All`), then
    // unwind their rotation one at a time (`Flow`).
    let finale = Block {
        combinator: Combinator::Chain,
        children: vec![
            AnimNode::block(Block {
                combinator: Combinator::All,
                children: cubes
                    .iter()
                    .map(|cube| {
                        scale_to(
                            cube.subject,
                            &mut values,
                            Vec3::splat(0.7),
                            cs(30),
                        )
                    })
                    .collect(),
            }),
            AnimNode::block(Block {
                combinator: Combinator::Flow(cs(8)),
                children: cubes
                    .iter()
                    .map(|cube| {
                        rotate_to(
                            cube.subject,
                            &mut values,
                            Quat::IDENTITY,
                            cs(40),
                        )
                    })
                    .collect(),
            }),
        ],
    };

    let animation = Block::chain(vec![
        AnimNode::block(scale_in),
        AnimNode::block(paired_rotate),
        AnimNode::block(race),
        AnimNode::block(finale),
    ]);

    // Every field the animation touches, at what the cubes spawn
    // holding. Not redundant with that spawn: baking reads off the
    // world, and an edit recompiles with the cubes wherever the last
    // run left them.
    let stage = Stage {
        subjects: cubes
            .iter()
            .map(|cube| Subject {
                id: cube.subject,
                fields: vec![
                    seed(
                        &mut values,
                        field_ref(path!(<Transform>::translation)),
                        Vec3::new(cube.x, 0.0, 0.0),
                    ),
                    seed(
                        &mut values,
                        field_ref(path!(<Transform>::rotation)),
                        Quat::IDENTITY,
                    ),
                    seed(
                        &mut values,
                        field_ref(path!(<Transform>::scale)),
                        Vec3::ZERO,
                    ),
                ],
            })
            .collect(),
    };

    let scene = AnimScene {
        stage,
        animation,
        values,
    };
    commands.insert_resource(moxie::EditorScene::new(
        MotionGfxScene(scene),
    ));
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera {
            clear_color: Color::srgb(0.02, 0.02, 0.04).into(),
            ..default()
        },
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 14.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
