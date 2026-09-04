//! The shape of a field's move to a new value: how long, along what
//! curve, and how the value itself is walked. A transition plays one
//! out. See
//! [`ElementMut::transition`](crate::ui::ElementMut::transition).

use core::time::Duration;

/// Interpolates between two `T` by a factor in `0..=1`.
pub type InterpFn<T> = fn(&T, &T, f32) -> T;

pub use motiongfx_interp::ease::EaseFn;
/// How a value walks between two of itself. What `#[elem(anim(...))]`
/// reaches for unless the field names its own `interp = ...`.
pub use motiongfx_interp::interpolation::Interpolation;

/// A field's curve to a new value: duration, easing, interpolation.
pub struct Tween<T> {
    /// Zero arrives on the next flush.
    pub duration: Duration,
    pub ease: EaseFn,
    pub interp: InterpFn<T>,
}

impl<T> Tween<T> {
    pub fn new(duration: Duration, interp: InterpFn<T>) -> Self {
        Self {
            duration,
            ease: motiongfx_interp::ease::linear,
            interp,
        }
    }

    pub fn secs(duration: f32, interp: InterpFn<T>) -> Self {
        Self::new(Duration::from_secs_f32(duration), interp)
    }

    pub fn ms(duration: u32, interp: InterpFn<T>) -> Self {
        Self::new(Duration::from_millis(duration as u64), interp)
    }

    pub fn ease(mut self, ease: EaseFn) -> Self {
        self.ease = ease;
        self
    }

    /// The eased `0..=1` progress at `elapsed`.
    pub(crate) fn at(&self, elapsed: Duration) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }
        let linear =
            elapsed.as_secs_f32() / self.duration.as_secs_f32();
        (self.ease)(linear.clamp(0.0, 1.0))
    }

}

// A derive would bound `T: Clone`; only the fn pointers get cloned.
impl<T> Clone for Tween<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Tween<T> {}
