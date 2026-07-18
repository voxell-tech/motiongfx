use core::any::TypeId;

use alloc::boxed::Box;
use field_path::field::UntypedField;

use crate::ThreadSafe;
use crate::subject::SubjectId;

mod id_registry;
mod table;

pub use id_registry::{IdRegistry, UId};
pub use table::{
    ActionBuilder, ActionId, ActionMarker, ActionTable,
    InterpActionBuilder,
};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct UntypedSubjectId {
    /// The [`TypeId`] of the [`SubjectId`].
    type_id: TypeId,
    /// The type-erased [`UId`] of the [`SubjectId`].
    uid: UId,
}

impl UntypedSubjectId {
    pub const PLACEHOLDER: Self =
        Self::placeholder_with_u64(u64::MAX);

    pub const fn new<I: SubjectId>(uid: UId) -> Self {
        Self {
            type_id: TypeId::of::<I>(),
            uid,
        }
    }

    pub const fn placeholder_with_u64(id: u64) -> Self {
        Self {
            type_id: TypeId::of::<()>(),
            uid: UId(id),
        }
    }

    pub const fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub const fn uid(&self) -> UId {
        self.uid
    }
}

/// Key that uniquely identifies a sequence of non-overlapping
/// actions.
///
/// Treated as immutable by convention: `track.rs` stores this as a
/// `HashMap` key, so it must never be mutated in place after
/// insertion (previously enforced at compile time by
/// `#[component(immutable)]`; now just a documented invariant).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct ActionKey {
    /// The subject Id of the action.
    subject_id: UntypedSubjectId,
    /// The source and target field related to the subject.
    field: UntypedField,
}

impl ActionKey {
    pub fn new(
        subject_id: UntypedSubjectId,
        field: UntypedField,
    ) -> Self {
        Self { subject_id, field }
    }

    pub fn subject_id(&self) -> &UntypedSubjectId {
        &self.subject_id
    }

    pub fn field(&self) -> &UntypedField {
        &self.field
    }
}

/// An action trait which consists of a function for getting
/// the target value based on an intial value.
pub trait Action<T>: ThreadSafe + Fn(&T) -> T {}

impl<T, U> Action<T> for U where U: ThreadSafe + Fn(&T) -> T {}

/// A storage value for an [`Action`].
pub struct ActionStorage<T> {
    pub action: Box<dyn Action<T>>,
}

impl<T> ActionStorage<T> {
    pub fn new(action: impl Action<T>) -> Self {
        Self {
            action: Box::new(action),
        }
    }
}

/// Function for interpolating a type based on a [`f32`] time.
pub type InterpFn<T> = fn(start: &T, end: &T, t: f32) -> T;

/// A storage value for a custom [`InterpFn`].
///
/// This can be optionally inserted alongside [`ActionStorage`]
/// to customize the action.
#[derive(Debug, Clone, Copy)]
pub struct InterpStorage<T>(pub InterpFn<T>);

/// Easing function on a [`f32`] time.
pub type EaseFn = fn(t: f32) -> f32;

/// A storage value for a custom [`EaseFn`].
///
/// This can be optionally inserted alongside [`ActionStorage`]
/// to customize the action.
#[derive(Debug, Clone, Copy)]
pub struct EaseStorage(pub EaseFn);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActionClip {
    pub id: ActionId,
    pub start: f32,
    pub duration: f32,
}

impl ActionClip {
    pub const fn new(id: ActionId, duration: f32) -> Self {
        Self {
            id,
            start: 0.0,
            duration,
        }
    }

    #[inline]
    pub fn end(&self) -> f32 {
        self.start + self.duration
    }
}

pub struct Segment<T> {
    /// The starting value.
    pub start: T,
    /// The ending value.
    pub end: T,
}

impl<T> Segment<T> {
    pub fn new(start: T, end: T) -> Self {
        Self { start, end }
    }
}

/// Determines how a [`Segment`] should be sampled.
#[derive(Debug, Clone, Copy)]
pub enum SampleMode {
    Start,
    End,
    Interp(f32),
}
