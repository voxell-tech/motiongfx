#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

pub mod action;
pub mod ease;
pub mod interpolation;
pub mod pipeline;
pub mod registry;
pub mod sequence;
pub mod subject;
pub mod timeline;
pub mod track;
pub mod world;

// Re-exports field_path as it is essential for motiongfx to work!
pub use field_path;

pub mod prelude {
    pub use field_path::field_accessor::FieldAccessor;

    pub use crate::ThreadSafe;
    pub use crate::action::{
        Action, ActionBuilder, ActionId, EaseFn, InterpActionBuilder,
        InterpFn,
    };
    pub use crate::ease;
    pub use crate::interpolation::Interpolation;
    pub use crate::path;
    pub use crate::pipeline::PipelineKey;
    pub use crate::registry::{
        AccessorRegistry, PipelineRegistry, Registry,
    };
    pub use crate::timeline::{Timeline, TimelineBuilder};
    pub use crate::track::{Track, TrackFragment, TrackOrdering};
    pub use crate::world::SubjectSource;
}

/// See [`field_path::field_accessor!`].
///
/// This macro just forwards the tokens to the mentioned macro.
///
/// ## Example
///
/// ```
/// use motiongfx::path;
///
/// struct Foo(u32);
///
/// let path = path!(<Foo>::0);
/// ```
#[macro_export]
macro_rules! path {
    ($($t:tt)*) => {
        $crate::field_path::field_accessor!($($t)*)
    };
}

/// Auto trait for types that implements [`Send`] + [`Sync`] +
/// `'static`.
pub trait ThreadSafe: Send + Sync + 'static {}

impl<T> ThreadSafe for T where T: Send + Sync + 'static {}
