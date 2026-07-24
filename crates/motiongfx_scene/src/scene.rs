//! The top-level [`Scene`]: the stage's initial state plus its
//! animation.

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::block::Block;

/// The whole serialized project: the initial stage and the animation
/// that drives it. `Id` and `V` are backend-chosen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene<Id, V> {
    pub stage: Stage<Id, V>,
    /// The root block. An empty timeline is `Block::chain([])`.
    pub animation: Block<Id, V>,
}

/// The initial state of the world: a flat, uniform set of subjects.
/// Deliberately no object-vs-asset split; that's a backend concern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stage<Id, V> {
    pub subjects: Vec<Subject<Id, V>>,
}

/// One animatable thing on the stage, addressed by a stable id.
/// `state` is opaque to core; the backend owns its shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Subject<Id, V> {
    pub id: Id,
    pub state: V,
}
