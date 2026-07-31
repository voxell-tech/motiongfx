//! The top-level [`Scene`]: the stage's initial state plus its
//! animation.

use alloc::vec::Vec;
use educe::Educe;
use serde::{Deserialize, Serialize};

use crate::backend::SceneBackend;
use crate::block::Block;
use crate::refs::FieldRef;

/// The whole serialized project: the initial stage and the animation
/// that drives it, plus the value pool both reference into. `B`
/// bundles the backend's chosen types; see [`SceneBackend`].
#[derive(Educe, Serialize, Deserialize)]
#[educe(
    Debug(bound(B::ValuePool: core::fmt::Debug)),
    Clone(bound(B::ValuePool: Clone)),
    PartialEq(bound(B::ValuePool: PartialEq))
)]
#[serde(bound = "")]
pub struct Scene<B: SceneBackend> {
    pub stage: Stage<B>,
    /// The root block. An empty timeline is `Block::chain([])`.
    pub animation: Block<B>,
    /// Every subject-state and action-argument value, referenced by
    /// [`SceneBackend::ValueId`] from `stage`/`animation`. See
    /// [`SceneBackend::ValuePool`].
    pub values: B::ValuePool,
}

/// Holds the initial value of every animated field, as a flat, uniform
/// set of subjects. Deliberately no object-vs-asset split; that's a
/// backend concern.
#[derive(Educe, Serialize, Deserialize)]
#[educe(Debug, Clone, PartialEq)]
#[serde(bound = "")]
pub struct Stage<B: SceneBackend> {
    pub subjects: Vec<Subject<B>>,
}

/// One animatable thing on the stage, addressed by a stable id.
#[derive(Educe, Serialize, Deserialize)]
#[educe(Debug, Clone, PartialEq)]
#[serde(bound = "")]
pub struct Subject<B: SceneBackend> {
    pub id: B::Id,
    /// Initial value of each field animated on this subject.
    pub fields: Vec<FieldSeed<B>>,
}

/// The initial value of one animation track.
///
/// Baking reads `prev` from the world, not the scene, so seed these in
/// with [`Scene::stage`] first. `value` references into
/// [`Scene::values`].
#[derive(Educe, Serialize, Deserialize)]
#[educe(Debug, Clone, PartialEq)]
#[serde(bound = "")]
pub struct FieldSeed<B: SceneBackend> {
    pub field: FieldRef,
    pub value: B::ValueId,
}
