//! The animation tree: nested [`Block`]s of [`Node`]s.
//!
//! A [`Block`] carries a [`Combinator`] for how its children combine;
//! a [`Node`] is a nested block, an action leaf, or a delayed wrapper.
//! Each maps 1:1 onto a `motiongfx` track combinator.

use alloc::{boxed::Box, vec::Vec};
use core::time::Duration;

use serde::{Deserialize, Serialize};

use crate::refs::{EaseRef, FieldRef, InterpRef, OpRef};

/// A group of [`Node`]s combined by one [`Combinator`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block<Id, V> {
    pub combinator: Combinator,
    pub children: Vec<Node<Id, V>>,
}

impl<Id, V> Block<Id, V> {
    /// A sequential `Chain` block; also the shape of an empty timeline.
    pub fn chain(children: Vec<Node<Id, V>>) -> Self {
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Node<Id, V> {
    Block(Block<Id, V>),
    Action(ActionCmd<Id, V>),
    /// Shifts `node` later by `offset`. (`delay`)
    Delayed {
        offset: Duration,
        node: Box<Node<Id, V>>,
    },
}

/// Applies `op(value)` to `subject.field` over `duration`, eased and
/// interpolated by name. No closures or Rust types, only names and an
/// opaque value; the registry reconstructs the typed action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionCmd<Id, V> {
    pub subject: Id,
    pub field: FieldRef,
    pub op: OpRef,
    pub value: V,
    pub duration: Duration,
    /// `None` = linear / default easing.
    pub ease: Option<EaseRef>,
    /// `None` = the field type's default interpolation.
    pub interp: Option<InterpRef>,
}
