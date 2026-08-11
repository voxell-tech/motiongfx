//! Demonstrates the [`MoxiePlugin`] timeline editor.
//!
//! A row of cubes animates through a single track containing several
//! actions. The editor docks a timeline panel at the bottom of the
//! window: use the play/pause button to control playback and drag on
//! the timeline to scrub. If the track is wider than the window, scroll
//! the panel horizontally to reveal the rest.

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
use motiongfx_scene::scene::{Scene as AnimScene, Stage};
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

/// Spawns a row of cubes and an [`EditorScene`] animating them: each
/// cube grows, then the whole row spins, staggered so the timeline
/// panel has plenty of nested blocks to show.
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
    let mut grow: Vec<AnimNode<Backend>> =
        Vec::with_capacity(CUBE_COUNT);
    let mut spin: Vec<AnimNode<Backend>> =
        Vec::with_capacity(CUBE_COUNT);

    for i in 0..CUBE_COUNT {
        let x = (i as f32) - (CUBE_COUNT as f32 - 1.0) * 0.5;
        let material = materials.add(StandardMaterial::from_color(
            palettes::tailwind::SKY_400,
        ));
        let uid = EntityUid::new();
        commands.spawn((
            uid,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(x * 1.5, 0.0, 0.0)
                .with_scale(Vec3::ZERO),
        ));
        let subject = SceneUid::Entity(uid);

        grow.push(AnimNode::action(ActionCmd {
            subject,
            field: field_ref(path!(<Transform>::scale)),
            op: AnimOp::To,
            value: values.insert(Vec3::ONE),
            duration: cs(60),
            ease: Some(AnimEase::CubicEaseInOut),
            interp: Some(AnimInterp::Linear),
        }));

        spin.push(AnimNode::action(ActionCmd {
            subject,
            field: field_ref(path!(<Transform>::rotation)),
            op: AnimOp::To,
            value: values
                .insert(Quat::from_rotation_y(std::f32::consts::PI)),
            duration: s(1),
            ease: Some(AnimEase::CubicEaseInOut),
            interp: Some(AnimInterp::Linear),
        }));
    }

    let animation = Block::chain(vec![
        AnimNode::block(Block {
            combinator: Combinator::Flow(cs(15)),
            children: grow,
        }),
        AnimNode::block(Block {
            combinator: Combinator::Flow(cs(10)),
            children: spin,
        }),
    ]);

    let scene = AnimScene {
        // Cubes already spawn with the right initial `Transform`
        // (scale zero), so there's nothing to seed here.
        stage: Stage {
            subjects: Vec::new(),
        },
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
