//! The animation tree: nested [`Block`]s of [`Node`]s.
//!
//! A [`Block`] carries a [`Combinator`] for how its children combine;
//! a [`Node`] is a nested block, an action leaf, or a delayed wrapper.
//! Each maps 1:1 onto a `motiongfx` track combinator.

use alloc::{boxed::Box, vec::Vec};
use core::time::Duration;

use educe::Educe;
use serde::{Deserialize, Serialize};

use crate::backend::SceneBackend;
use crate::refs::FieldRef;

/// A group of [`Node`]s combined by one [`Combinator`].
///
/// `Block`/`Node` are mutually recursive (`Node::Block(Block<B>)`,
/// `Block.children: Vec<Node<B>>`), which `educe`'s automatic bound
/// inference can't see across - explicit `bound(...)` overrides below
/// give it the real per-field bounds instead of falling back to
/// requiring `B` itself implement each trait. Only `B::Value` needs a
/// bound stated here: `B::Id: SubjectId` and `B::OpId`/`B::InterpId`/
/// `B::EaseId: Key` already guarantee `Debug`/`Clone`/`Eq` (which
/// implies `PartialEq`) as supertraits, so those hold without restating
/// them.
#[derive(Educe, Serialize, Deserialize)]
#[educe(
    Debug(bound(B::Value: core::fmt::Debug)),
    Clone(bound(B::Value: Clone)),
    PartialEq(bound(B::Value: PartialEq))
)]
#[serde(bound(
    serialize = "B::Id: Serialize, B::Value: Serialize, B::OpId: Serialize, B::InterpId: Serialize, B::EaseId: Serialize",
    deserialize = "B::Id: Deserialize<'de>, B::Value: Deserialize<'de>, B::OpId: Deserialize<'de>, B::InterpId: Deserialize<'de>, B::EaseId: Deserialize<'de>"
))]
pub struct Block<B: SceneBackend> {
    pub combinator: Combinator,
    pub children: Vec<Node<B>>,
}

impl<B: SceneBackend> Block<B> {
    /// A sequential `Chain` block; also the shape of an empty timeline.
    pub fn chain(children: Vec<Node<B>>) -> Self {
        Self {
            combinator: Combinator::Chain,
            children,
        }
    }
}

/// How a [`Block`]'s children combine in time. Each maps onto a
/// `motiongfx::track` combinator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Combinator {
    /// Sequential; children laid end to end. (`ord_chain`)
    Chain,
    /// Simultaneous; children share a start, wait for all. (`ord_all`)
    All,
    /// Simultaneous; wait for any. (`ord_any`)
    Any,
    /// Staggered starts, one `delay` apart. (`ord_flow`)
    Flow(Duration),
}

/// A member of a [`Block`]: a nested block, an action leaf, or a
/// delayed wrapper.
///
/// Explicit `bound(...)` overrides for the same reason as
/// [`Block`] - `Node` is directly self-referential via
/// `Delayed.node: Box<Node<B>>`.
#[derive(Educe, Serialize, Deserialize)]
#[educe(
    Debug(bound(B::Value: core::fmt::Debug)),
    Clone(bound(B::Value: Clone)),
    PartialEq(bound(B::Value: PartialEq))
)]
#[serde(bound(
    serialize = "B::Id: Serialize, B::Value: Serialize, B::OpId: Serialize, B::InterpId: Serialize, B::EaseId: Serialize",
    deserialize = "B::Id: Deserialize<'de>, B::Value: Deserialize<'de>, B::OpId: Deserialize<'de>, B::InterpId: Deserialize<'de>, B::EaseId: Deserialize<'de>"
))]
pub enum Node<B: SceneBackend> {
    Block(Block<B>),
    Action(ActionCmd<B>),
    /// Shifts `node` later by `offset`. (`delay`)
    Delayed {
        offset: Duration,
        node: Box<Node<B>>,
    },
}

/// Applies `op(value)` to `subject.field` over `duration`, eased and
/// interpolated by name. No closures or Rust types, only names and an
/// opaque value; the registry reconstructs the typed action.
#[derive(Educe, Serialize, Deserialize)]
#[educe(Debug, Clone, PartialEq)]
#[serde(bound(
    serialize = "B::Id: Serialize, B::Value: Serialize, B::OpId: Serialize, B::InterpId: Serialize, B::EaseId: Serialize",
    deserialize = "B::Id: Deserialize<'de>, B::Value: Deserialize<'de>, B::OpId: Deserialize<'de>, B::InterpId: Deserialize<'de>, B::EaseId: Deserialize<'de>"
))]
pub struct ActionCmd<B: SceneBackend> {
    pub subject: B::Id,
    pub field: FieldRef,
    pub op: B::OpId,
    pub value: B::Value,
    pub duration: Duration,
    /// `None` = linear / default easing.
    pub ease: Option<B::EaseId>,
    /// `None` = the field type's default interpolation.
    pub interp: Option<B::InterpId>,
}
