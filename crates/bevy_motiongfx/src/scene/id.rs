//! Stable ids for scene subjects, and their materialization to
//! runtime handles.
//!
//! `Entity` is generational and gets reassigned on every scene load,
//! so it cannot be what a serialized [`Scene`](motiongfx_scene::scene::Scene)
//! refers to. [`EntityUid`] is a plain `Uuid` v4: stable across
//! save/load, and attached as a [`Component`] to every entity it
//! materializes as - so the reverse (`Entity` -> `EntityUid`) lookup
//! is just a normal component read, not a second map to keep in sync.
//! [`SceneUidMap`] is how [`BevyWorld`](crate::world::BevyWorld)'s
//! [`SubjectSource<EntityUid, S>`](motiongfx::world::SubjectSource)
//! impl (see `crate::world`) resolves an `EntityUid` to whatever
//! `Entity` it is currently spawned as, before delegating to the same
//! `Entity`-keyed access `BevyWorld` already provides. Nothing outside
//! this map may assume an `Entity` is stable across a save/reload
//! cycle - only the `EntityUid` is.

use core::fmt;

use bevy_ecs::prelude::*;
use bevy_platform::collections::HashMap;
use motiongfx_scene::backend::IntoSubjectId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Component,
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
pub struct EntityUid(Uuid);

impl EntityUid {
    /// Generates a new random (v4) id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// See [`Uuid::nil()`].
    pub fn nil() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for EntityUid {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EntityUid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// A scene's subject id: which kind of thing it names is explicit in
/// the format itself, not inferred from which field targets it.
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
pub enum SceneUid {
    Entity(EntityUid),
}

impl IntoSubjectId<EntityUid> for SceneUid {
    fn into_subject_id(self) -> Option<EntityUid> {
        match self {
            SceneUid::Entity(id) => Some(id),
        }
    }
}

/// Maps a scene's [`EntityUid`]s to the `Entity` each is currently
/// materialized as. Store as a resource in the same [`World`] the
/// scene's entities are spawned into.
///
/// Kept in sync by [`on_add_entity_uid`]/[`on_remove_entity_uid`] -
/// there's no manual `insert`/`remove`; spawning or despawning the
/// [`EntityUid`] component is the only way in or out.
#[derive(Resource, Default)]
pub struct SceneUidMap {
    to_entity: HashMap<EntityUid, Entity>,
}

impl SceneUidMap {
    pub fn entity(&self, id: EntityUid) -> Option<Entity> {
        self.to_entity.get(&id).copied()
    }
}

/// Records `id`'s entity in [`SceneUidMap`] whenever an [`EntityUid`]
/// is added to it.
pub(crate) fn on_add_entity_uid(
    trigger: On<Add, EntityUid>,
    query: Query<&EntityUid>,
    mut uid_map: ResMut<SceneUidMap>,
) {
    if let Ok(id) = query.get(trigger.entity) {
        uid_map.to_entity.insert(*id, trigger.entity);
    }
}

/// Removes `id`'s entry from [`SceneUidMap`] whenever an [`EntityUid`]
/// is removed from it (including on despawn).
pub(crate) fn on_remove_entity_uid(
    trigger: On<Remove, EntityUid>,
    query: Query<&EntityUid>,
    mut uid_map: ResMut<SceneUidMap>,
) {
    if let Ok(id) = query.get(trigger.entity) {
        uid_map.to_entity.remove(id);
    }
}
