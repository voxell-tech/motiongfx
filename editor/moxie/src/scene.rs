//! [`EditorScene`]: the editor's authoritative `Scene<Backend>`. It is
//! what gets edited (and, later, saved/loaded) - the `Timeline` in
//! [`MotionGfxManager`] is a compiled, disposable view of it,
//! rebuilt by `recompile_dirty_scene` whenever it changes.

use bevy::prelude::*;
use bevy_motiongfx::prelude::*;
use bevy_motiongfx::scene::asset::MotionGfxScene;
use bevy_motiongfx::scene::backend::{
    BackendRegistry, default_scene_registry,
};
/// The project: a scene plus the registry that resolves its names,
/// and whether it's changed since the last compile.
///
/// The action panel edits the tree, the timeline panel's row layout
/// reads it, and `recompile_dirty_scene` turns it back into a
/// timeline whenever it has changed.
///
/// Public (unlike most of this crate's state) because the example
/// binaries build it directly, in place of `motiongfx`'s imperative
/// track builder.
#[derive(Resource)]
pub struct EditorScene {
    scene: MotionGfxScene,
    registry: BackendRegistry,
    dirty: bool,
}

impl EditorScene {
    /// Wraps `scene`, marked dirty so `recompile_dirty_scene` compiles
    /// it on the very next run.
    pub fn new(scene: MotionGfxScene) -> Self {
        Self {
            scene,
            registry: default_scene_registry(),
            dirty: true,
        }
    }

    pub(crate) fn scene(&self) -> &MotionGfxScene {
        &self.scene
    }

    /// The scene, to change. Taking the borrow marks it dirty, so
    /// there is no way to edit it and forget to say so.
    pub(crate) fn edit(&mut self) -> &mut MotionGfxScene {
        self.dirty = true;
        &mut self.scene
    }
}

/// Recompiles [`EditorScene`] into a fresh [`BevyTimeline`] whenever it's
/// dirty, replacing whichever timeline the (single) player entity was
/// showing - preserving its playhead across the swap - or spawning
/// that entity on the very first compile.
pub(crate) fn recompile_dirty_scene(world: &mut World) {
    world.resource_scope::<EditorScene, _>(
        |world, mut editor_scene| {
            if !editor_scene.dirty {
                return;
            }
            editor_scene.dirty = false;

            world.resource_scope::<MotionGfxManager, _>(
                |world, mut manager| {
                    let mut q_players =
                        world.query::<(Entity, &TimelineId)>();
                    let existing = q_players
                        .iter(world)
                        .next()
                        .map(|(entity, &id)| (entity, id));
                    let playhead = existing.and_then(|(_, id)| {
                        manager
                            .get_timeline(&id)
                            .map(|timeline| timeline.target_time())
                    });
                    if let Some((_, old_id)) = existing {
                        manager.remove_timeline(&old_id);
                    }

                    let new_id = editor_scene
                        .scene
                        .compile(&editor_scene.registry, &mut manager)
                        .expect("editor scene should compile");

                    editor_scene
                        .scene
                        .stage(&editor_scene.registry, world)
                        .expect("editor scene should stage");

                    if let Some(time) = playhead
                        && let Some(timeline) =
                            manager.get_timeline_mut(&new_id)
                    {
                        timeline.set_target_track(0);
                        timeline.set_target_time(time);
                    }

                    match existing {
                        Some((entity, _)) => {
                            world.entity_mut(entity).insert(new_id);
                        }
                        None => {
                            world.spawn((
                                new_id,
                                RealtimePlayer::new(),
                            ));
                        }
                    }
                },
            );
        },
    );
}
