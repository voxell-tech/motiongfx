//! Stable ids for scene subjects, and their materialization to
//! runtime handles.
//!
//! `Entity` is generational and gets reassigned on every scene load,
//! so it cannot be what a serialized [`Scene`](motiongfx_scene::scene::Scene)
//! refers to. [`SceneId`] is a plain `Uuid` v4: stable across
//! save/load. [`SceneEntityMap`] is how
//! [`BevyWorld`](crate::world::BevyWorld)'s
//! [`SubjectSource<SceneId, S>`](motiongfx::world::SubjectSource) impl
//! (see `crate::world`) resolves a `SceneId` to whatever `Entity` it
//! is currently spawned as, before delegating to the same
//! `Entity`-keyed access `BevyWorld` already provides. Nothing outside
//! this map may assume an `Entity` is stable across a save/reload
//! cycle - only the `SceneId` is.

use core::fmt;

use bevy_ecs::prelude::*;
use bevy_platform::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct SceneId(Uuid);

impl SceneId {
    /// Generates a new random (v4) id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SceneId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SceneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Bidirectional mapping between a scene's [`SceneId`]s and the
/// `Entity` each is currently materialized as. Store as a resource in
/// the same [`World`] the scene's entities are spawned into.
#[derive(Resource, Default)]
pub struct SceneEntityMap {
    to_entity: HashMap<SceneId, Entity>,
    to_scene_id: HashMap<Entity, SceneId>,
}

impl SceneEntityMap {
    pub fn insert(&mut self, id: SceneId, entity: Entity) {
        self.to_entity.insert(id, entity);
        self.to_scene_id.insert(entity, id);
    }

    pub fn remove_by_scene_id(
        &mut self,
        id: SceneId,
    ) -> Option<Entity> {
        let entity = self.to_entity.remove(&id)?;
        self.to_scene_id.remove(&entity);
        Some(entity)
    }

    pub fn remove_by_entity(
        &mut self,
        entity: Entity,
    ) -> Option<SceneId> {
        let id = self.to_scene_id.remove(&entity)?;
        self.to_entity.remove(&id);
        Some(id)
    }

    pub fn entity(&self, id: SceneId) -> Option<Entity> {
        self.to_entity.get(&id).copied()
    }

    pub fn scene_id(&self, entity: Entity) -> Option<SceneId> {
        self.to_scene_id.get(&entity).copied()
    }
}
