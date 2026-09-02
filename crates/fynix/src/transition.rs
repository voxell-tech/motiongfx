//! What a field does on its way to a new value.
//!
//! A transition is an overlay, never a write: the element keeps the
//! value the cascade gave it, and the kernel keeps the one the backend
//! is currently showing. See
//! [`ElementMut::transition`](crate::ui::ElementMut::transition).

/// How a value travels. Carried rather than looked up through a
/// trait, so declaring an overlay says nothing about the backend.
pub type LerpFn<T> = fn(&T, &T, f32) -> T;

pub use motiongfx_interp::ease::EaseFn;

/// How long a field takes to arrive, along what curve, and how the
/// value itself is walked.
pub struct Transition<T> {
    /// Seconds. Zero arrives on the next flush.
    pub duration: f32, // TODO: Change to Duration.
    pub ease: EaseFn,
    pub lerp: LerpFn<T>,
}

impl<T> Transition<T> {
    pub fn secs(duration: f32, lerp: LerpFn<T>) -> Self {
        Self {
            duration,
            ease: motiongfx_interp::ease::linear,
            lerp,
        }
    }

    pub fn ms(duration: u32, lerp: LerpFn<T>) -> Self {
        Self::secs(duration as f32 / 1000.0, lerp)
    }

    pub fn ease(mut self, ease: EaseFn) -> Self {
        self.ease = ease;
        self
    }

    /// Where the curve stands after `elapsed`.
    pub(crate) fn at(&self, elapsed: f32) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        (self.ease)((elapsed / self.duration).clamp(0.0, 1.0))
    }

    pub(crate) fn done(&self, elapsed: f32) -> bool {
        elapsed >= self.duration
    }
}

// Derived, these would ask `T` for what only the pointers need.
impl<T> Clone for Transition<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Transition<T> {}
