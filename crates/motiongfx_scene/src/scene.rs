//! The top-level [`Scene`]: the stage's initial state plus its
//! animation.

use alloc::vec::Vec;
use educe::Educe;
use serde::{Deserialize, Serialize};

use crate::backend::SceneBackend;
use crate::block::Block;

/// The whole serialized project: the initial stage and the animation
/// that drives it. `B` bundles the backend's chosen types; see
/// [`SceneBackend`].
#[derive(Educe, Serialize, Deserialize)]
#[educe(Debug, Clone, PartialEq)]
#[serde(bound = "")]
pub struct Scene<B: SceneBackend> {
    pub stage: Stage<B>,
    /// The root block. An empty timeline is `Block::chain([])`.
    pub animation: Block<B>,
}

/// The initial state of the world: a flat, uniform set of subjects.
/// Deliberately no object-vs-asset split; that's a backend concern.
#[derive(Educe, Serialize, Deserialize)]
#[educe(Debug, Clone, PartialEq)]
#[serde(bound = "")]
pub struct Stage<B: SceneBackend> {
    pub subjects: Vec<Subject<B>>,
}

/// One animatable thing on the stage, addressed by a stable id.
/// `state` is opaque to core; the backend owns its shape.
#[derive(Educe, Serialize, Deserialize)]
#[educe(Debug, Clone, PartialEq)]
#[serde(bound = "")]
pub struct Subject<B: SceneBackend> {
    pub id: B::Id,
    pub state: B::Value,
}
