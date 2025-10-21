//! [Motion Canvas]: https://motioncanvas.io/
//! [Manim]: https://www.manim.community/
//! [Bevy]: https://bevyengine.org/
//! [Vello]: https://github.com/linebender/vello
//! [Typst]: https://typst.app
//! [`World`]: bevy_ecs::world::World
//! [`SubjectId`]: subject::SubjectId
//! [`ActionWorld`]: action::ActionWorld
//! [`Track`]: track::Track
//! [`Timeline`]: timeline::Timeline
//! [`FieldAccessorRegistry`]: field_path::accessor::FieldAccessorRegistry
//! [`PipelineRegistry`]: pipeline::PipelineRegistry
//! [`Pipeline`]: pipeline::Pipeline
//! [`PipelineKey`]: pipeline::PipelineKey
//!
//! # MotionGfx
//!
//! **MotionGfx** is a backend-agnostic motion graphics framework
//! built on top of the [Bevy] ECS. It provides a modular foundation
//! for procedural animations.
//!
//! It also leverages the [`field_path`] crate to enable
//! *type-erased field access*, allowing animation data to be
//! dynamically linked to any structure without requiring concrete
//! type information at compile time.
//!
//! ## Running Examples
//!
//! ```bash
//! # Clone the repo and run the examples
//! git clone https://github.com/voxell-tech/motiongfx
//! cd motiongfx
//! cargo run --example hello_world
//! ```
//!
//! ## Core Concepts
//!
//! MotionGfx is organized around a few fundamental building blocks:
//!
//! - **[`SubjectId`]**: A generic Id that points to the actual
//!   subject that is meant to be animated by the actions.
//!
//! - **[`ActionWorld`]**: A container that stores all active
//!   animation actions within a timeline via the Bevy ECS's
//!   [`World`]. Each action defines how a property evolves over time.
//!
//! - **[`Track`]**: Represents sequences of actions in chronological
//!   order, each with a defined start time and duration. Tracks
//!   ensure that actions within them are played in the correct
//!   temporal order.
//!
//! - **[`Timeline`]**: The top-level structure that coordinates a
//!   sequence of tracks and their associated [`ActionWorld`]. Each
//!   track acts like a checkpoint, allowing animations to be grouped
//!   into discrete blocks.
//!
//! - **[`FieldAccessorRegistry`]**: Maintains a mapping between
//!   animatable fields and their corresponding accessors, enabling
//!   MotionGfx to read and write values on arbitrary data structures
//!   in a type-safe yet dynamic way.
//!
//! - **[`PipelineRegistry`]**: Associates [`PipelineKey`]s with
//!   concrete [`Pipeline`] implementations. Pipelines handle the
//!   baking of actions and the sampling of animation segments for
//!   playback or preview.
//!
//! In short, a timeline holds all the actions and subject Ids via an
//! action world and stores the timing of each actions within blocks
//! of tracks. Then, when it comes to animating subjects, the timeline
//! reaches out to the pipeline registry and accessor registry to
//! perform baking and sampling.
//!
//! ## Using MotionGfx
//!
//! MotionGfx on its own is extremely simple to use, below is an
//! example of how to use it. (Read the comments!)
//!
//! ```
//! use std::collections::HashMap;
//!
//! use motiongfx::prelude::*;
//!
//! // First, we have to initialize a subject world and the
//! // registries.
//! #[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
//! struct Id(u32);
//! #[derive(Debug, Clone, Copy)]
//! struct Subject(f32);
//! type SubjectWorld = HashMap<Id, Subject>;
//!
//! let mut subject_world = SubjectWorld::new();
//! let mut accessor_registry = FieldAccessorRegistry::new();
//! let mut pipeline_registry =
//!     PipelineRegistry::<SubjectWorld>::new();
//!
//! // The accessor registry should contain accessors to the fields in
//! // the subjects. In our case, it's just the first field in
//! // the tuple struct: `Subject::0`.
//!
//! accessor_registry.register_typed(
//!     field!(<Subject>::0),
//!     accessor!(<Subject>::0),
//! );
//!
//! // Similarly, the pipeline registry shoiud contain pipelines to
//! // bake and sample the fields in the subjects.
//!
//! pipeline_registry.register_unchecked(
//!     PipelineKey::new::<Id, Subject, f32>(),
//!     Pipeline::new(
//!         |world, ctx| {
//!             ctx.bake::<Id, Subject, f32>(|id| world.get(&id));
//!         },
//!         |world, ctx| {
//!             ctx.sample::<Id, Subject, f32>(
//!                 |id, target, accessor| {
//!                     if let Some(x) = world.get_mut(&id) {
//!                         *accessor.get_mut(x) = target;
//!                     }
//!                 },
//!             );
//!         },
//!     ),
//! );
//!
//! // Now that the registries are complete, we can start adding
//! // subjects into the subject world.
//!
//! subject_world.insert(Id(1), Subject(0.0));
//!
//! // A timeline can only be created via the `TimelineBuilder`.
//!
//! let mut builder = TimelineBuilder::new();
//!
//! let track = builder
//!     // Creates the action.
//!     .act(Id(1), field!(<Subject>::0), |x| x + 10.0)
//!     // Adds an interpolation method.
//!     .with_interp(|&a, &b, t| a + (b - a) * t)
//!     // Specifies the duration of the action.
//!     .play(1.0)
//!     // Compiles into a track.
//!     .compile();
//!
//! // Adds the track to the builder.
//! builder.add_tracks(track);
//! // And compile it into a timeline.
//! let mut timeline = builder.compile();
//! // The timeline needs to be baked once before sampling can happen.
//! timeline.bake_actions(
//!     &pipeline_registry,
//!     &subject_world,
//!     &accessor_registry,
//! );
//!
//! // Let's visualize the current state of the subject world before
//! // the sampling happens.
//! println!("Before: {:?}", subject_world);
//!
//! // We fast forward the timeline.
//! timeline.set_target_time(0.5);
//! // Actions need to be queued before it can be sampled.
//! // The queued actions are stored internally.
//! timeline.queue_actions();
//! timeline.sample_queued_actions(
//!     &pipeline_registry,
//!     &mut subject_world,
//!     &accessor_registry,
//! );
//!
//! // Visualize the state of the subject world after the sampling.
//! println!("After:  {:?}", subject_world);
//! ```
//!
//! ## Inspirations and Similar Projects
//!
//! - [Motion Canvas]
//! - [Manim]

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
