//! The editor binary: [`MoxiePlugin`] over a starting scene.
//!
//! Until it can open a saved one, that scene is built here. Two rows
//! of shapes run through five phases - pop in together, rotate in
//! staggered pairs, race each other, drift apart, then settle - which
//! between them mix and nest every [`Combinator`], so the timeline
//! panel has a real tree to show. The shapes hang under a row each,
//! so the hierarchy panel has one too.

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
use moxie::{MoxiePlugin, SceneRoot};
// Aliased: `Node` and `Scene` both also name `bevy`/`bevy_ui` types
// pulled in above.
use motiongfx_scene::block::{
    ActionCmd, Block, Combinator, Node as AnimNode,
};
use motiongfx_scene::refs::FieldRef;
// Aliased: `Subject` is also what this file calls one of its own
// animated things.
use motiongfx_scene::scene::{
    FieldSeed, Scene as AnimScene, Stage, Subject as StageSubject,
};
use motiongfx_scene::value::ValueColumn;

/// Shapes in a row, and rows in the scene.
const PER_ROW: usize = 3;

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

/// One animated thing: its subject id and where it starts, so a later
/// phase can move it relative to that.
struct Subject {
    id: SceneUid,
    start: Vec3,
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

/// Spawns two rows of shapes and an [`EditorScene`] animating them
/// through five phases, deliberately mixing and nesting every
/// [`Combinator`]:
///
/// 1. **Scale in** (`All`) - every shape pops in at once.
/// 2. **Paired rotate** (`Flow` of `All` pairs) - shapes rotate two at
///    a time, each pair staggered after the last.
/// 3. **Race** (`Any` of an action and a `Chain`) - two shapes chase
///    the same finish line; the phase ends the instant the faster one
///    does, leaving the slower one mid-flight.
/// 4. **Drift** (`All`) - the rows themselves move apart. Nothing
///    animates the shapes here: they come along because they hang
///    under a row, which is what the hierarchy panel is showing.
/// 5. **Finale** (`Chain` of `All` then `Flow`) - every shape settles
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
    let mut values = ValuePool::default();

    // The scene root, so top-level subjects have an order to sit in.
    // Spawned before everything else so it exists before any subject
    // that needs to parent under it.
    let root = commands
        .spawn((
            SceneRoot,
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    // Flat shapes are meshed in the XY plane, so they face the camera
    // to begin with - and are double sided, or a rotation would turn
    // them edge-on and then away.
    let rows = [
        (
            "Solids",
            1.2,
            palettes::tailwind::SKY_400,
            [
                meshes.add(Cuboid::default()),
                meshes.add(Sphere::new(0.6)),
                meshes.add(Torus::new(0.22, 0.45)),
            ],
        ),
        (
            "Flats",
            -1.2,
            palettes::tailwind::AMBER_400,
            [
                meshes.add(Rectangle::new(1.1, 1.1)),
                meshes.add(Circle::new(0.62)),
                meshes.add(RegularPolygon::new(0.68, 6)),
            ],
        ),
    ];

    let mut parents = Vec::new();
    let mut shapes = Vec::new();

    for (name, y, color, row_meshes) in rows {
        let origin = Vec3::new(0.0, y, 0.0);
        let uid = EntityUid::new();
        let parent = commands
            .spawn((
                uid,
                Name::new(name),
                Transform::from_translation(origin),
                Visibility::default(),
                ChildOf(root),
            ))
            .id();
        parents.push(Subject {
            id: SceneUid::Entity(uid),
            start: origin,
        });

        let material = materials.add(StandardMaterial {
            base_color: color.into(),
            double_sided: true,
            cull_mode: None,
            ..default()
        });

        for (i, mesh) in row_meshes.into_iter().enumerate() {
            let x = ((i as f32) - (PER_ROW as f32 - 1.0) * 0.5) * 2.0;
            let start = Vec3::new(x, 0.0, 0.0);
            let uid = EntityUid::new();

            commands.spawn((
                uid,
                Name::new(format!("{name} {i}")),
                Mesh3d(mesh),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(start)
                    .with_scale(Vec3::ZERO),
                ChildOf(parent),
            ));
            shapes.push(Subject {
                id: SceneUid::Entity(uid),
                start,
            });
        }
    }

    // Phase 1 - every shape pops in together.
    let scale_in = Block {
        combinator: Combinator::All,
        children: shapes
            .iter()
            .map(|shape| {
                scale_to(shape.id, &mut values, Vec3::ONE, cs(40))
            })
            .collect(),
    };

    // Phase 2 - shapes rotate two at a time (an `All` pair), each pair
    // staggered after the last (a `Flow` of those pairs).
    let paired_rotate = Block {
        combinator: Combinator::Flow(cs(25)),
        children: shapes
            .chunks(2)
            .map(|pair| {
                AnimNode::block(Block {
                    combinator: Combinator::All,
                    children: pair
                        .iter()
                        .map(|shape| {
                            rotate_to(
                                shape.id,
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

    // Phase 3 - two shapes race for the same finish line: a lone slow
    // hop against a `Chain` of two quick hops. `Any` ends the instant
    // the faster one does, so the slow hop is left unfinished.
    let race = Block {
        combinator: Combinator::Any,
        children: vec![
            move_to(
                shapes[0].id,
                &mut values,
                shapes[0].start + Vec3::Y * 1.5,
                s(2),
            ),
            AnimNode::block(Block {
                combinator: Combinator::Chain,
                children: vec![
                    move_to(
                        shapes[1].id,
                        &mut values,
                        shapes[1].start + Vec3::Y,
                        cs(50),
                    ),
                    move_to(
                        shapes[1].id,
                        &mut values,
                        shapes[1].start,
                        cs(50),
                    ),
                ],
            }),
        ],
    };

    // Phase 4 - the rows themselves move apart. Nothing here names a
    // shape: they follow because they hang under a row.
    let drift = Block {
        combinator: Combinator::All,
        children: parents
            .iter()
            .map(|row| {
                move_to(
                    row.id,
                    &mut values,
                    row.start + Vec3::Y * row.start.y.signum() * 0.6,
                    cs(60),
                )
            })
            .collect(),
    };

    // Phase 5 - settle every shape's scale together (`All`), then
    // unwind their rotation one at a time (`Flow`).
    let finale = Block {
        combinator: Combinator::Chain,
        children: vec![
            AnimNode::block(Block {
                combinator: Combinator::All,
                children: shapes
                    .iter()
                    .map(|shape| {
                        scale_to(
                            shape.id,
                            &mut values,
                            Vec3::splat(0.7),
                            cs(30),
                        )
                    })
                    .collect(),
            }),
            AnimNode::block(Block {
                combinator: Combinator::Flow(cs(8)),
                children: shapes
                    .iter()
                    .map(|shape| {
                        rotate_to(
                            shape.id,
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
        AnimNode::block(drift),
        AnimNode::block(finale),
    ]);

    // Every field the animation touches, at what the shapes spawn
    // holding. Not redundant with that spawn: baking reads off the
    // world, and an edit recompiles with them wherever the last run
    // left them. A row starts at full scale; a shape pops in from
    // nothing.
    let stage = Stage {
        subjects: shapes
            .iter()
            .map(|shape| (shape, Vec3::ZERO))
            .chain(parents.iter().map(|row| (row, Vec3::ONE)))
            .map(|(subject, scale)| StageSubject {
                id: subject.id,
                fields: vec![
                    seed(
                        &mut values,
                        field_ref(path!(<Transform>::translation)),
                        subject.start,
                    ),
                    seed(
                        &mut values,
                        field_ref(path!(<Transform>::rotation)),
                        Quat::IDENTITY,
                    ),
                    seed(
                        &mut values,
                        field_ref(path!(<Transform>::scale)),
                        scale,
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
