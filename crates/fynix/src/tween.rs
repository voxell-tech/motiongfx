//! The shape of a field's move to a new value: how long, along what
//! curve, and how the value itself is walked. A transition plays one
//! out. See
//! [`ElementMut::transition`](crate::ui::ElementMut::transition).

use core::time::Duration;

/// Interpolates between two `T` by a factor in `0..=1`.
pub type LerpFn<T> = fn(&T, &T, f32) -> T;

pub use motiongfx_interp::ease::EaseFn;

/// A field's curve to a new value: duration, easing, interpolation.
pub struct Tween<T> {
    /// Zero arrives on the next flush.
    pub duration: Duration,
    pub ease: EaseFn,
    pub lerp: LerpFn<T>,
}

impl<T> Tween<T> {
    pub fn new(duration: Duration, lerp: LerpFn<T>) -> Self {
        Self {
            duration,
            ease: motiongfx_interp::ease::linear,
            lerp,
        }
    }

    pub fn secs(duration: f32, lerp: LerpFn<T>) -> Self {
        Self::new(Duration::from_secs_f32(duration), lerp)
    }

    pub fn ms(duration: u32, lerp: LerpFn<T>) -> Self {
        Self::new(Duration::from_millis(duration as u64), lerp)
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

    pub(crate) fn done(&self, elapsed: Duration) -> bool {
        elapsed >= self.duration
    }
}

// A derive would bound `T: Clone`; only the fn pointers get cloned.
impl<T> Clone for Tween<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Tween<T> {}
