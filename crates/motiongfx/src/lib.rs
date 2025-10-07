//! [Motion Canvas]: https://motioncanvas.io/
//! [Manim]: https://www.manim.community/
//! [Bevy]: https://bevyengine.org/
//! [Vello]: https://github.com/linebender/vello
//! [Typst]: https://typst.app
//!
//! MotionGfx is a motion graphics creation tool.
//! It is highly inspired by [Motion Canvas] & [Manim].

#![no_std]

extern crate alloc;

pub mod accessor;
pub mod action;
pub mod ease;
pub mod field;
pub mod pipeline;
pub mod sequence;
pub mod subject;
pub mod timeline;
pub mod track;

/// Auto trait for types that implements [`Send`] + [`Sync`] + `'static`.
pub trait ThreadSafe: Send + Sync + 'static {}

impl<T> ThreadSafe for T where T: Send + Sync + 'static {}
