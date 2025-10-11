//! The [`SubjectId`] trait represents an identifier for a "subject"
//! within a timeline system. A subject is the entity, object, or role
//! that actions are applied to during playback. This abstraction
//! allows MotionGfx to remain agnostic about what uniquely identifies
//! a subject across different backends.
//!
//! By standardizing on [`SubjectId`], actions can generically
//! reference and manipulate their intended subjects without assuming
//! a specific engine or ID representation.

use core::fmt::Debug;
use core::hash::Hash;

use crate::ThreadSafe;

/// An auto trait bound for the identifier of the subject.
///
/// The identifier should be thread safe, lightweight, and supports
/// debug, copy, comparison, and hash.
pub trait SubjectId:
    ThreadSafe + Debug + Copy + Clone + Eq + Ord + Hash
{
}

impl<T> SubjectId for T where
    T: ThreadSafe + Debug + Copy + Clone + Eq + Ord + Hash
{
}
