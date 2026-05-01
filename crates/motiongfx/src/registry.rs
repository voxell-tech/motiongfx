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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path;
    use crate::pipeline::SubjectSource;

    // ── Helper types ──────────────────────────────────────────────────────────

    /// A minimal subject holding one f32 field.
    #[derive(Clone)]
    struct Subject {
        pub value: f32,
    }

    /// A minimal world that stores a single Subject.
    struct SimpleWorld {
        subject: Subject,
    }

    impl SubjectSource<usize, Subject> for SimpleWorld {
        fn get_source(&self, _id: usize) -> Option<&Subject> {
            Some(&self.subject)
        }

        fn apply_source<R>(
            &mut self,
            _id: usize,
            f: impl FnOnce(&mut Subject) -> R,
        ) -> Option<R> {
            Some(f(&mut self.subject))
        }
    }

    // ── AccessorRegistry ─────────────────────────────────────────────────────

    #[test]
    fn accessor_registry_new_is_empty() {
        let registry = AccessorRegistry::new();
        // An unregistered field should return None.
        let field_acc = path!(<Subject>::value);
        let untyped = field_acc.field.untyped();
        assert!(registry.get::<Subject, f32>(&untyped).is_none());
    }

    #[test]
    fn accessor_registry_register_then_get_returns_accessor() {
        let mut registry = AccessorRegistry::new();
        let field_acc = path!(<Subject>::value);
        let untyped = field_acc.field.untyped();

        registry.register(field_acc);

        let accessor = registry.get::<Subject, f32>(&untyped);
        assert!(accessor.is_some(), "Accessor should be found after registration");
    }

    #[test]
    fn accessor_registry_get_works_on_registered_field() {
        let mut registry = AccessorRegistry::new();
        let field_acc = path!(<Subject>::value);
        let untyped = field_acc.field.untyped();

        registry.register(field_acc);

        let accessor = registry.get::<Subject, f32>(&untyped).unwrap();
        let mut subject = Subject { value: 0.0 };
        *accessor.get_mut(&mut subject) = 42.0;
        assert_eq!(subject.value, 42.0);
    }

    #[test]
    fn accessor_registry_register_is_idempotent() {
        // Registering the same field twice should not panic or change the
        // underlying entry.
        let mut registry = AccessorRegistry::new();
        let field_acc1 = path!(<Subject>::value);
        let field_acc2 = path!(<Subject>::value);
        let untyped = field_acc1.field.untyped();

        registry.register(field_acc1);
        registry.register(field_acc2); // second call should be a no-op

        // Registry should still work correctly after idempotent registration.
        let accessor = registry.get::<Subject, f32>(&untyped);
        assert!(accessor.is_some());
    }

    #[test]
    fn accessor_registry_get_returns_none_for_wrong_type() {
        // Registering as Subject->f32 but querying as Subject->u32 returns None.
        let mut registry = AccessorRegistry::new();
        let field_acc = path!(<Subject>::value);
        let untyped = field_acc.field.untyped();

        registry.register(field_acc);

        // Querying with mismatched target type returns None.
        let wrong = registry.get::<Subject, u32>(&untyped);
        assert!(wrong.is_none());
    }

    // ── PipelineRegistry ─────────────────────────────────────────────────────

    #[test]
    fn pipeline_registry_register_then_contains_pipeline() {
        let mut registry = PipelineRegistry::new();
        // Before registration, bake should return false.
        let key = PipelineKey::new::<SimpleWorld, usize, Subject, f32>();
        assert!(!registry.pipelines.contains_key(&key));

        registry.register::<SimpleWorld, usize, Subject, f32>();

        assert!(registry.pipelines.contains_key(&key));
    }

    #[test]
    fn pipeline_registry_register_is_idempotent() {
        let mut registry = PipelineRegistry::new();
        registry.register::<SimpleWorld, usize, Subject, f32>();
        registry.register::<SimpleWorld, usize, Subject, f32>(); // second call skipped
        // Should still have exactly one entry for the key.
        let key = PipelineKey::new::<SimpleWorld, usize, Subject, f32>();
        assert!(registry.pipelines.contains_key(&key));
    }

    #[test]
    fn pipeline_registry_bake_returns_false_for_wrong_world_type() {
        let mut registry = PipelineRegistry::new();
        registry.register::<SimpleWorld, usize, Subject, f32>();

        // A key using a different world type should not match.
        struct OtherWorld;
        impl SubjectSource<usize, Subject> for OtherWorld {
            fn get_source(&self, _id: usize) -> Option<&Subject> { None }
            fn apply_source<R>(&mut self, _id: usize, _f: impl FnOnce(&mut Subject) -> R) -> Option<R> { None }
        }

        // The key for OtherWorld is not registered, so bake should return false.
        let wrong_key = PipelineKey::new::<OtherWorld, usize, Subject, f32>();
        assert!(!registry.pipelines.contains_key(&wrong_key));
    }

    #[test]
    fn pipeline_registry_bake_returns_false_when_key_not_found() {
        let registry = PipelineRegistry::new();
        // Key is not registered at all.
        let key = PipelineKey::new::<SimpleWorld, usize, Subject, f32>();
        assert!(!registry.pipelines.contains_key(&key));
    }

    // ── Registry (combined) ───────────────────────────────────────────────────

    #[test]
    fn registry_default_is_empty() {
        let registry = Registry::default();
        let field_acc = path!(<Subject>::value);
        let untyped = field_acc.field.untyped();
        assert!(registry.accessor.get::<Subject, f32>(&untyped).is_none());
    }

    #[test]
    fn registry_register_populates_both_sub_registries() {
        let mut registry = Registry::new();
        let field_acc = path!(<Subject>::value);
        let untyped = field_acc.field.untyped();

        registry.register::<SimpleWorld, usize, Subject, f32>(
            path!(<Subject>::value),
        );

        // Accessor registry should have the field.
        assert!(registry.accessor.get::<Subject, f32>(&untyped).is_some());

        // Pipeline registry should have the key.
        let key = PipelineKey::new::<SimpleWorld, usize, Subject, f32>();
        assert!(registry.pipeline.pipelines.contains_key(&key));
    }

    #[test]
    fn registry_create_builder_returns_builder() {
        let mut registry = Registry::new();
        // create_builder should not panic and return a valid builder.
        let _builder = registry.create_builder::<SimpleWorld>();
    }
}
