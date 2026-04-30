use bevy_platform::collections::HashMap;
use field_path::accessor::{Accessor, UntypedAccessor};
use field_path::field::{Field, UntypedField};

use crate::pipeline::{
    Pipeline, PipelineHandle, PipelineKey, PipelineUntyped,
};
use crate::subject::SubjectId;

pub struct Registry {
    pub accessor: AccessorRegistry,
    pub pipeline: PipelineRegistry,
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
        field: Field<S, T>,
        accessor: Accessor<S, T>,
    ) {
        let untyped_field = field.untyped();
        if self.accessors.contains_key(&untyped_field) {
            return;
        }

        self.accessors.insert(untyped_field, accessor.untyped());
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

    pub fn get(&self, key: &PipelineKey) -> Option<&PipelineUntyped> {
        self.pipelines.get(key)
    }

    /// Register a pipeline. Skips pipelines already registered.
    pub fn register<
        W: 'static,
        I: SubjectId,
        S: 'static,
        T: 'static,
    >(
        &mut self,
        handle: PipelineHandle<W, I, S, T>,
        pipeline: Pipeline<I, S, T>,
    ) -> &mut Self {
        let key = handle.as_key();
        if self.pipelines.contains_key(&key) {
            return self;
        }

        self.pipelines.insert(key, pipeline.untyped());
        self
    }
}

impl Default for PipelineRegistry {
    fn default() -> Self {
        Self::new()
    }
}
