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
/// Every field `Block`/`Node`/`ActionCmd` reference through `B`
/// directly (`B::Id`, `B::OpId`, `B::EaseId`, `B::InterpId`) is
/// already unconditionally `Debug + Clone + PartialEq` via
/// `SubjectId`'s/`Key`'s own supertraits, and so is `B::ValueId` (used
/// by `ActionCmd::value`) - so these impls never actually depend on a
/// backend choice the way the old
/// `SceneBackend::Value` did. `bound(false)` (no extra where-clause)
/// is deliberate, not just "the simplest option that compiles": a
/// bound referencing `Node<B>`/`Block<B>`/`ActionCmd<B>` here would
/// make the derived impls mutually conditional on each other with no
/// base case, which overflows the trait solver.
#[derive(Educe, Serialize, Deserialize)]
#[educe(
    Debug(bound(false)),
    Clone(bound(false)),
    PartialEq(bound(false))
)]
#[serde(bound = "")]
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
    Flow(#[serde(with = "crate::duration")] Duration),
}

/// A member of a [`Block`]: a nested block, an action leaf, or a
/// delayed wrapper.
#[derive(Educe, Serialize, Deserialize)]
#[educe(
    Debug(bound(false)),
    Clone(bound(false)),
    PartialEq(bound(false))
)]
#[serde(bound = "")]
pub enum Node<B: SceneBackend> {
    Block(Block<B>),
    Action(ActionCmd<B>),
    /// Shifts `node` later by `offset`. (`delay`)
    Delayed {
        #[serde(with = "crate::duration")]
        offset: Duration,
        node: Box<Node<B>>,
    },
}

/// Applies `op(value)` to `subject.field` over `duration`, eased and
/// interpolated by name. No closures or Rust types, only names and an
/// opaque value; the registry reconstructs the typed action.
#[derive(Educe, Serialize, Deserialize)]
#[educe(Debug, Clone, PartialEq)]
#[serde(bound = "")]
pub struct ActionCmd<B: SceneBackend> {
    pub subject: B::Id,
    pub field: FieldRef,
    pub op: B::OpId,
    pub value: B::ValueId,
    #[serde(with = "crate::duration")]
    pub duration: Duration,
    /// `None` = linear / default easing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ease: Option<B::EaseId>,
    /// `None` = the field type's default interpolation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interp: Option<B::InterpId>,
}
