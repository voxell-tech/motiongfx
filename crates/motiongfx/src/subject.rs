//! The [`SubjectId`] trait represents an identifier for a "subject"
//! within an animation or timeline system. A subject is the entity,
//! object, or role that actions are applied to during playback.
//! This abstraction allows MotionGfx to remain agnostic about what
//! uniquely identifies a subject across different worlds or
//! backends.
//!
//! By standardizing on [`SubjectId`], actions can generically
//! reference and manipulate their intended subjects without assuming
//! a specific engine or ID representation.

use core::fmt::Debug;
use core::hash::Hash;

use crate::ThreadSafe;

// TODO: Rename to something else? Like `ActorId`?
pub trait SubjectId:
    ThreadSafe
    + Debug
    + Copy
    + Clone
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + Hash
{
}

impl<T> SubjectId for T where
    T: ThreadSafe
        + Debug
        + Copy
        + Clone
        + PartialEq
        + Eq
        + PartialOrd
        + Ord
        + Hash
{
}
