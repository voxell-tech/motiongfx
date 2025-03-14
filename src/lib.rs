//! [Bevy]: https://bevyengine.org/
//! [Vello]: https://github.com/linebender/vello
//! [Typst]: https://typst.app
//! [Motion Canvas]: https://motioncanvas.io/
//! [Manim]: https://www.manim.community/
//! [`Action`]: motiongfx_core::action::Action
//! [`Sequence`]: motiongfx_core::sequence::Sequence
//!
//! Bevy MotionGfx is a motion graphics creation tool. It is highly inspired by [Motion Canvas] & [Manim].
//! The core technologies that is being used are:
//! - [Bevy]: Renderer, entity component system, etc.
//! - [Vello]: Vector graphics rasterization.
//! - [Typst]: Document to svg.
//!
//! The core of this crate is made up of [`Action`] and [`Sequence`].
//!
//! # Action
//! [`Action`] is the building blocks of this crate. It [stores](prelude::Action::new) the smallest unit of work that can be done in an animation:
//! - The target entity.
//! - The component of that entity.
//! - The begin and end state of the component.
//! - The interpolation function to use for interpolating between the begin and end state.
//!
//! # Sequence
//! A [`Sequence`] is made up of multiple [`Action`]s. You can think of it as a group of actions. A [`Sequence`] also defines the order of [`Action`]s through the use of [action ordering functions](motiongfx_core::sequence).

use bevy::prelude::*;

pub use motiongfx_core;

#[cfg(feature = "common")]
pub use motiongfx_common;

pub mod prelude {
    pub use motiongfx_core::prelude::*;

    #[cfg(feature = "common")]
    pub use motiongfx_common::prelude::*;
}

pub struct MotionGfxPlugin;

impl Plugin for MotionGfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(motiongfx_core::MotionGfxCorePlugin);
        #[cfg(feature = "common")]
        app.add_plugins(motiongfx_common::MotionGfxCommonPlugin);
    }
}
