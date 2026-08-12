//! What a field does on its way to a new value.
//!
//! A transition is an overlay, never a write: the element keeps the
//! value the cascade gave it, and the kernel keeps the one the backend
//! is currently showing. See
//! [`ElementMut::transition`](crate::ui::ElementMut::transition).

/// How a value travels. Carried rather than looked up through a
/// trait, so declaring a lane says nothing about the backend.
pub type LerpFn<T> = fn(&T, &T, f32) -> T;

/// What a curve is: `0.0` at the start, `1.0` at the end.
pub type EaseFn = fn(f32) -> f32;

/// The curves a transition is likely to want. Any `fn(f32) -> f32`
/// does.
pub mod ease {
    /// No curve at all.
    pub fn linear(t: f32) -> f32 {
        t
    }

    /// Fast, then settling, which is what an interaction wants.
    pub fn cubic_out(t: f32) -> f32 {
        let rest = 1.0 - t;
        1.0 - rest * rest * rest
    }

    pub fn cubic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            let rest = -2.0 * t + 2.0;
            1.0 - rest * rest * rest / 2.0
        }
    }
}

/// [`LerpFn`]s for the types this crate knows about. A backend brings
/// its own for its own.
pub mod lerp {
    macro_rules! lerp {
        ($($name:ident: $ty:ty),* $(,)?) => {
            $(
                pub fn $name(a: &$ty, b: &$ty, t: f32) -> $ty {
                    let (a, b) = (*a as f32, *b as f32);
                    (a + (b - a) * t) as $ty
                }
            )*
        };
    }

    lerp!(float: f32, int: i32, uint: u32, byte: u8);
}

/// How long a field takes to arrive, along what curve, and how the
/// value itself is walked.
pub struct Transition<T> {
    /// Seconds. Zero arrives on the next flush.
    pub duration: f32,
    pub ease: EaseFn,
    pub lerp: LerpFn<T>,
}

impl<T> Transition<T> {
    pub fn secs(duration: f32, lerp: LerpFn<T>) -> Self {
        Self {
            duration,
            ease: ease::linear,
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
