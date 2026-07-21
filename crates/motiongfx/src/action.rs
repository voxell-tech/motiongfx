use core::any::TypeId;
use core::time::Duration;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionClip {
    pub id: ActionId,
    pub start: Duration,
    pub duration: Duration,
}

impl ActionClip {
    pub const fn new(id: ActionId, duration: Duration) -> Self {
        Self {
            id,
            start: Duration::ZERO,
            duration,
        }
    }

    /// Saturating, so an absurd authored duration degrades to a clamped
    /// timeline rather than a panic deep in playback.
    #[inline]
    pub fn end(&self) -> Duration {
        self.start.saturating_add(self.duration)
    }

    /// Normalized progress of `time` through this clip, in
    /// \[0.0..=1.0\].
    ///
    /// Zero-duration spacer clips report `1.0` rather than `NaN`.
    #[inline]
    pub fn progress(&self, time: Duration) -> f32 {
        let span = self.duration.as_secs_f64();

        if span == 0.0 {
            return 1.0;
        }

        let elapsed = time.saturating_sub(self.start).as_secs_f64();
        (elapsed / span).clamp(0.0, 1.0) as f32
    }
}

#[cfg(test)]
mod clip_tests {
    use super::*;

    const fn clip(start: u64, duration: u64) -> ActionClip {
        ActionClip {
            id: ActionId::PLACEHOLDER,
            start: Duration::from_millis(start),
            duration: Duration::from_millis(duration),
        }
    }

    #[test]
    fn progress_spans_the_clip() {
        let clip = clip(1000, 2000);

        assert_eq!(clip.progress(Duration::from_millis(1000)), 0.0);
        assert_eq!(clip.progress(Duration::from_millis(2000)), 0.5);
        assert_eq!(clip.progress(Duration::from_millis(3000)), 1.0);
    }

    /// `end()` runs on every queue pass, so a saturated duration must
    /// not panic there.
    #[test]
    fn end_saturates_instead_of_overflowing() {
        let clip = ActionClip {
            id: ActionId::PLACEHOLDER,
            start: Duration::MAX,
            duration: Duration::MAX,
        };

        assert_eq!(clip.end(), Duration::MAX);
    }

    #[test]
    fn progress_clamps_outside_the_clip() {
        let clip = clip(1000, 2000);

        assert_eq!(clip.progress(Duration::ZERO), 0.0);
        assert_eq!(clip.progress(Duration::from_secs(60)), 1.0);
    }

    /// A `NaN` here would poison the interpolated value.
    #[test]
    fn zero_duration_clip_reports_completion() {
        let t = clip(1000, 0).progress(Duration::from_millis(1000));

        assert!(!t.is_nan());
        assert_eq!(t, 1.0);
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
