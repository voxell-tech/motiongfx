#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

pub mod action;
pub mod ease;
pub mod pipeline;
pub mod sequence;
pub mod subject;
pub mod timeline;
pub mod track;

// Re-exports field_path as it is essential for motiongfx to work!
pub use field_path;

pub mod prelude {
    pub use field_path::accessor::{
        Accessor, FieldAccessorRegistry, UntypedAccessor, accessor,
    };
    pub use field_path::field::{Field, UntypedField, field};

    pub use crate::ThreadSafe;
    pub use crate::action::{
        Action, ActionBuilder, ActionId, EaseFn, InterpActionBuilder,
        InterpFn,
    };
    pub use crate::ease;
    pub use crate::pipeline::{
        BakeCtx, Pipeline, PipelineKey, PipelineRegistry, SampleCtx,
    };
    pub use crate::timeline::{Timeline, TimelineBuilder};
    pub use crate::track::{Track, TrackFragment, TrackOrdering};
}

/// Auto trait for types that implements [`Send`] + [`Sync`] +
/// `'static`.
pub trait ThreadSafe: Send + Sync + 'static {}

impl<T> ThreadSafe for T where T: Send + Sync + 'static {}
