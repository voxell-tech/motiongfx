use core::any::TypeId;

use field_path::field::UntypedField;
use field_path::lens::{Lens, UntypedLens};
use field_path::path::Path;
use hashbrown::HashMap;

use crate::ThreadSafe;
use crate::pipeline::bake::BakeClipCtx;
use crate::pipeline::sample::SampleCtx;
use crate::pipeline::{
    Pipeline, PipelineHandle, PipelineKey, PipelineUntyped,
};
use crate::prelude::{SubjectSource, TimelineBuilder};
use crate::subject::SubjectId;

pub struct Registry {
    pub lens: LensRegistry,
    pub pipeline: PipelineRegistry,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            lens: LensRegistry::new(),
            pipeline: PipelineRegistry::new(),
        }
    }

    pub fn register<W, I, S, T>(&mut self, path: Path<S, T>)
    where
        W: SubjectSource<I, S> + 'static,
        I: SubjectId,
        S: Clone + ThreadSafe,
        T: Clone + ThreadSafe,
    {
        self.lens.register(path);
        self.pipeline.register::<W, I, S, T>();
    }

    /// Create a [`TimelineBuilder`] for a specific `W` world.
    pub fn create_builder<W: 'static>(
        &mut self,
    ) -> TimelineBuilder<'_, W> {
        TimelineBuilder::new(self)
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LensRegistry {
    lenses: HashMap<UntypedField, UntypedLens>,
}

impl LensRegistry {
    pub fn new() -> Self {
        Self {
            lenses: HashMap::new(),
        }
    }

    /// Registers a [`Path`] pair.
    /// Skips fields already registered.
    #[inline]
    pub fn register<S: 'static, T: 'static>(
        &mut self,
        path: Path<S, T>,
    ) {
        let untyped_field = path.field.untyped();
        if self.lenses.contains_key(&untyped_field) {
            return;
        }

        self.lenses.insert(untyped_field, path.lens.untyped());
    }

    /// Retrieve a typed [`Lens`] from the registry.
    pub fn get<S: 'static, T: 'static>(
        &self,
        field: &UntypedField,
    ) -> Option<Lens<S, T>> {
        self.lenses.get(field)?.typed()
    }
}

impl Default for LensRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PipelineRegistry {
    pipelines: HashMap<PipelineKey, PipelineUntyped>,
}

impl PipelineRegistry {
    pub fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
        }
    }

    pub(crate) fn bake_clip<W: 'static>(
        &self,
        key: &PipelineKey,
        ctx: BakeClipCtx<W>,
    ) -> bool {
        if key.world_id() != TypeId::of::<W>() {
            return false;
        }

        if let Some(pipeline) = self.pipelines.get(key) {
            // SAFETY: verified above that key.world_id == TypeId::of::<W>().
            unsafe { pipeline.bake_clip(ctx) };
            return true;
        }

        false
    }

    pub(crate) fn sample<W: 'static>(
        &self,
        key: &PipelineKey,
        ctx: SampleCtx<W>,
    ) -> bool {
        if key.world_id() != TypeId::of::<W>() {
            return false;
        }

        if let Some(pipeline) = self.pipelines.get(key) {
            // SAFETY: verified above that key.world_id == TypeId::of::<W>().
            unsafe { pipeline.sample(ctx) };
            return true;
        }

        false
    }

    /// Register a [`Pipeline`].
    /// Skips pipelines already registered.
    pub fn register<W, I, S, T>(&mut self) -> &mut Self
    where
        W: SubjectSource<I, S> + 'static,
        I: SubjectId,
        S: Clone + ThreadSafe,
        T: Clone + ThreadSafe,
    {
        let key = PipelineHandle::<W, I, S, T>::new().as_key();
        if self.pipelines.contains_key(&key) {
            return self;
        }

        self.pipelines
            .insert(key, Pipeline::<W, I, S, T>::new().untyped());
        self
    }
}

impl Default for PipelineRegistry {
    fn default() -> Self {
        Self::new()
    }
}
