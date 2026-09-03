//! The shape of a field's move to a new value: how long, along what
//! curve, and how the value itself is walked. A transition plays one
//! out. See
//! [`ElementMut::transition`](crate::ui::ElementMut::transition).

/// Interpolates between two `T` by a factor in `0..=1`.
pub type LerpFn<T> = fn(&T, &T, f32) -> T;

pub use motiongfx_interp::ease::EaseFn;

/// A field's curve to a new value: duration, easing, interpolation.
pub struct Tween<T> {
    /// Seconds. Zero arrives on the next flush.
    pub duration: f32, // TODO: Change to Duration.
    pub ease: EaseFn,
    pub lerp: LerpFn<T>,
}

impl<T> Tween<T> {
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

    /// The eased `0..=1` progress at `elapsed`.
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

// A derive would bound `T: Clone`; only the fn pointers get cloned.
impl<T> Clone for Tween<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Tween<T> {}
