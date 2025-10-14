//! [`Bevy`]: https://bevyengine.org/
//! [`Motion Canvas`]: https://motioncanvas.io/
//! [`Manim`]: https://www.manim.community/
//! [`Action`]: motiongfx_core::action::Action
//! [`Sequence`]: motiongfx_core::sequence::Sequence
//! [`Timeline`]: motiongfx_core::timeline::Timeline
//!
//! Bevy MotionGfx is a motion graphics creation tool in [`Bevy`]. It is highly inspired by [`Motion Canvas`] & [`Manim`].
//! The core of this crate is made up of [`Action`], [`Sequence`], and [`Timeline`].
//!
//! # Action
//! [`Action`] is the building blocks of this crate. It [stores](prelude::Action::new) the smallest unit of work that can be done in an animation:
//! - The target entity.
//! - The component of that entity.
//! - The begin and end state of the component.
//! - The interpolation function to use for interpolating between the begin and end state.
//!
//! # Sequence
//! A [`Sequence`] is made up of multiple [`Action`]s. You can think of it as a group of actions. A [`Sequence`] also defines the order of [`Action`]s through the use of [action ordering functions](motiongfx_core::sequence#functions).
//!
//! # Timeline
//! A [`Timeline`] acts as a sequence player. The sole purpose of the timeline is to step through the [`Action`]s in the [`Sequence`]s attached to the [`Timeline`] and play the animations accordingly.

pub mod prelude {
    pub use motiongfx_bevy::prelude::*;
    pub use motiongfx_core::prelude::*;
    pub use motiongfx_vello::prelude::*;
}
