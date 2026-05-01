use core::any::TypeId;

use bevy_platform::collections::HashMap;
use field_path::accessor::{Accessor, UntypedAccessor};
use field_path::field::UntypedField;
use field_path::field_accessor::FieldAccessor;

use crate::ThreadSafe;
use crate::pipeline::{
    BakeCtx, Pipeline, PipelineHandle, PipelineKey, PipelineUntyped,
    SampleCtx,
};
use crate::prelude::{SubjectSource, TimelineBuilder};
use crate::subject::SubjectId;

pub struct Registry {
    pub accessor: AccessorRegistry,
    pub pipeline: PipelineRegistry,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            accessor: AccessorRegistry::new(),
            pipeline: PipelineRegistry::new(),
        }
    }

    pub fn register<W, I, S, T>(
        &mut self,
        field_acc: FieldAccessor<S, T>,
    ) where
        W: SubjectSource<I, S> + 'static,
        I: SubjectId,
        S: 'static,
        T: Clone + ThreadSafe,
    {
        self.accessor.register(field_acc);
        self.pipeline.register::<W, I, S, T>();
    }

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

pub struct AccessorRegistry {
    accessors: HashMap<UntypedField, UntypedAccessor>,
}

impl AccessorRegistry {
    pub fn new() -> Self {
        Self {
            accessors: HashMap::new(),
        }
    }

    /// Registers a field-accessor pair. Skips fields already registered.
    #[inline]
    pub fn register<S: 'static, T: 'static>(
        &mut self,
        field_acc: FieldAccessor<S, T>,
    ) {
        let untyped_field = field_acc.field.untyped();
        if self.accessors.contains_key(&untyped_field) {
            return;
        }

        self.accessors
            .insert(untyped_field, field_acc.accessor.untyped());
    }

    /// Retrieve a typed [`Accessor`] from the registry.
    pub fn get<S: 'static, T: 'static>(
        &self,
        field: &UntypedField,
    ) -> Option<Accessor<S, T>> {
        self.accessors.get(field)?.typed()
    }
}

impl Default for AccessorRegistry {
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

    pub(crate) fn bake<W: 'static>(
        &self,
        key: &PipelineKey,
        ctx: BakeCtx<W>,
    ) -> bool {
        if key.world_id() != TypeId::of::<W>() {
            return false;
        }

        if let Some(pipeline) = self.pipelines.get(key) {
            // SAFETY: verified above that key.world_id == TypeId::of::<W>().
            unsafe { pipeline.bake(ctx) };
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

    /// Register a pipeline. Skips pipelines already registered.
    pub fn register<W, I, S, T>(&mut self) -> &mut Self
    where
        W: SubjectSource<I, S> + 'static,
        I: SubjectId,
        S: 'static,
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
