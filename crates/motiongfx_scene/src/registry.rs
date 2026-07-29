//! The reconstruction boundary: maps scene names to typed runtime
//! closures. Filled by the backend at startup; see [`SceneRegistry`].

use core::marker::PhantomData;

use alloc::boxed::Box;

use hashbrown::HashMap;
use typarena::type_table::TypeTable;

use motiongfx::ThreadSafe;
use motiongfx::action::{Action, EaseFn, InterpFn};
use motiongfx::field_path::field::UntypedField;
use motiongfx::prelude::*;
use motiongfx::registry::Registry;
use motiongfx::world::SubjectSource;

use crate::backend::SceneBackend;
use crate::block::ActionCmd;
use crate::error::CompileError;
use crate::refs::{FieldRef, TypeName};
use crate::value::ValueColumn;

/// Resolves one `S`/`T`-typed field into a [`TrackFragment`], stored by
/// [`FieldRef`]. The action itself (looked up by `T` alone, not `S`; see
/// [`SceneRegistry::build_action`] and its `action_resolvers` map) doesn't
/// belong here - only the field-accessor step needs `S`.
trait FieldResolver<B: SceneBackend> {
    fn build(
        &self,
        cmd: &ActionCmd<B>,
        registry: &SceneRegistry<B>,
        values: &B::ValuePool,
        builder: &mut TimelineBuilder<'_, B::World>,
    ) -> Result<TrackFragment, CompileError<B>>;
}

struct ConcreteFieldResolver<B, S, T> {
    #[expect(clippy::type_complexity)]
    _marker: PhantomData<fn() -> (B, S, T)>,
}

impl<B, S, T> FieldResolver<B> for ConcreteFieldResolver<B, S, T>
where
    B: SceneBackend,
    B::World: SubjectSource<B::Id, S>,
    B::ValuePool: ValueColumn<B::ValueId, T>,
    S: 'static,
    T: ThreadSafe + Clone,
{
    fn build(
        &self,
        cmd: &ActionCmd<B>,
        registry: &SceneRegistry<B>,
        values: &B::ValuePool,
        builder: &mut TimelineBuilder<'_, B::World>,
    ) -> Result<TrackFragment, CompileError<B>> {
        let untyped_field = registry.resolve_field(&cmd.field)?;

        // Verify type match and get the typed accessor.
        let accessor = builder
            .registry()
            .accessor
            .get::<S, T>(&untyped_field)
            .ok_or_else(|| CompileError::TypeMismatch {
                type_name: core::any::type_name::<T>(),
                field: cmd.field.clone(),
            })?;

        // Reconstruct the typed Field and FieldAccessor.
        let field =
            untyped_field.typed::<S, T>().ok_or_else(|| {
                CompileError::TypeMismatch {
                    type_name: core::any::type_name::<T>(),
                    field: cmd.field.clone(),
                }
            })?;

        let field_acc = FieldAccessor::new(field, accessor);

        // Pulled from the pool *before* op resolution: by the time
        // `build_action` runs, `T` is already concrete, so the op
        // builder closure never needs to extract it from an opaque
        // value itself (there is no opaque value - see `crate::value`).
        let value = values
            .get(cmd.value)
            .ok_or(CompileError::UnknownValue(cmd.value))?;
        let action = registry.build_action::<T>(cmd.op, value)?;

        // Only known here, where `T` is concrete: pull the named interp
        // out of the type-erased map, or fall back to step.
        let interp_fn = registry.resolve_interp::<T>(&cmd.interp)?;
        let ease = registry.resolve_ease(&cmd.ease);

        let mut tb = builder
            .act_builder(cmd.subject, field_acc, action)
            .with_interp(interp_fn);

        if let Some(ease) = ease {
            tb = tb.with_ease(ease);
        }

        Ok(tb.play(cmd.duration))
    }
}

type BuildAction<T> =
    Box<dyn Fn(&T) -> Box<dyn Action<T>> + Send + Sync>;

type FieldResolverBox<B> = Box<dyn FieldResolver<B> + Send + Sync>;

type FieldRegistrar = Box<dyn Fn(&mut Registry) + Send + Sync>;

/// The bridge between scene names and runtime closures.
///
/// `B` bundles the backend's chosen types; see [`SceneBackend`]. Fill
/// via [`Self::register_field`], [`Self::register_op`], and
/// optionally [`Self::register_ease`]/[`Self::register_interp`].
pub struct SceneRegistry<B: SceneBackend> {
    /// Keyed by [`FieldRef`]; columns are `UntypedField`,
    /// [`FieldResolverBox`], and [`FieldRegistrar`].
    fields: TypeTable<FieldRef>,
    action_resolvers: TypeTable<B::OpId>,
    eases: HashMap<B::EaseId, EaseFn>,
    interps: TypeTable<B::InterpId>,
}

impl<B: SceneBackend> SceneRegistry<B> {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self {
            fields: TypeTable::new(),
            action_resolvers: TypeTable::new(),
            eases: HashMap::new(),
            interps: TypeTable::new(),
        }
    }

    /// Registers a field mapping.
    ///
    /// `type_name` + `path` form the [`FieldRef`] used in serialized
    /// [`ActionCmd`]s. `field_acc` is installed into the runtime
    /// `Registry` at [`compile`](crate::compile::compile) time.
    pub fn register_field<S, T>(
        &mut self,
        type_name: TypeName,
        path: impl Into<Box<str>>,
        field_acc: FieldAccessor<S, T>,
    ) where
        B::World: SubjectSource<B::Id, S>,
        B::ValuePool: ValueColumn<B::ValueId, T>,
        S: 'static,
        T: ThreadSafe + Clone,
    {
        let field_ref = FieldRef {
            type_name,
            path: path.into(),
        };
        let untyped = field_acc.field.untyped();
        self.fields
            .insert::<UntypedField>(field_ref.clone(), untyped);
        self.fields.insert::<FieldResolverBox<B>>(
            field_ref.clone(),
            Box::new(ConcreteFieldResolver::<B, S, T> {
                _marker: PhantomData,
            }),
        );

        // `FieldAccessor`'s derived `Copy` needs `S: Copy, T: Copy`, but
        // `Field`/`Accessor` alone are unconditionally `Copy` - capture
        // those instead to keep this closure `Fn`, not `FnOnce`.
        let field = field_acc.field;
        let accessor = field_acc.accessor;
        self.fields.insert::<FieldRegistrar>(
            field_ref,
            Box::new(move |runtime: &mut Registry| {
                runtime.register::<B::World, B::Id, S, T>(
                    FieldAccessor::new(field, accessor),
                );
            }),
        );
    }

    /// Installs every registered field's accessor into the runtime
    /// [`Registry`].
    pub(crate) fn install_accessors(
        &self,
        runtime_registry: &mut Registry,
    ) {
        for (_, registrar) in self.fields.iter::<FieldRegistrar>() {
            registrar(runtime_registry);
        }
    }

    /// Registers an op by name for a value type `T`.
    ///
    /// Keyed by `T` alone, not by any field's owning type `S`: the same
    /// `"to"`/`"by"` registered for `T = f32` covers every field of
    /// every `S` whose value type is `f32`. `build_action` receives the
    /// already-resolved concrete value directly (pulled from the value
    /// pool by [`ConcreteFieldResolver::build`](struct.ConcreteFieldResolver.html)
    /// before this is ever called) - no opaque value type to extract
    /// from, unlike the old `SceneBackend::Value`.
    pub fn register_op<T, F>(&mut self, op: B::OpId, build_action: F)
    where
        T: ThreadSafe + Clone,
        F: Fn(&T) -> Box<dyn Action<T>> + ThreadSafe,
    {
        let build_action: BuildAction<T> = Box::new(build_action);
        self.action_resolvers
            .insert::<BuildAction<T>>(op, build_action);
    }

    /// Registers an easing function by name.
    pub fn register_ease(&mut self, name: B::EaseId, ease: EaseFn) {
        self.eases.insert(name, ease);
    }

    /// Registers an interpolation function by name.
    pub fn register_interp<T>(
        &mut self,
        name: B::InterpId,
        interp: InterpFn<T>,
    ) where
        T: 'static,
    {
        self.interps.insert::<InterpFn<T>>(name, interp);
    }

    pub(crate) fn resolve_field(
        &self,
        field_ref: &FieldRef,
    ) -> Result<UntypedField, CompileError<B>> {
        self.fields
            .get::<UntypedField>(field_ref)
            .copied()
            .ok_or_else(|| {
                CompileError::UnknownField(field_ref.clone())
            })
    }

    pub(crate) fn resolve_ease(
        &self,
        ease: &Option<B::EaseId>,
    ) -> Option<EaseFn> {
        ease.as_ref().and_then(|name| self.eases.get(name).copied())
    }

    /// `None` falls back to step interpolation; an unregistered
    /// `Some(name)` is [`CompileError::UnknownInterp`].
    pub(crate) fn resolve_interp<T>(
        &self,
        interp: &Option<B::InterpId>,
    ) -> Result<InterpFn<T>, CompileError<B>>
    where
        T: Clone + 'static,
    {
        match interp {
            None => Ok(
                |a: &T, b: &T, t: f32| {
                    if t < 1.0 { a.clone() } else { b.clone() }
                },
            ),
            Some(name) => self
                .interps
                .get::<InterpFn<T>>(name)
                .copied()
                .ok_or(CompileError::UnknownInterp(*name)),
        }
    }

    /// Builds the action for `op` under a concrete `T`, or
    /// [`CompileError::UnknownOp`] if `op` isn't registered for this `T`.
    pub(crate) fn build_action<T>(
        &self,
        op: B::OpId,
        value: &T,
    ) -> Result<Box<dyn Action<T>>, CompileError<B>>
    where
        T: 'static,
    {
        self.action_resolvers
            .get::<BuildAction<T>>(&op)
            .map(|build_action| build_action(value))
            .ok_or_else(|| {
                CompileError::UnknownOp(
                    core::any::type_name::<T>(),
                    op,
                )
            })
    }

    pub(crate) fn resolve_op(
        &self,
        cmd: &ActionCmd<B>,
        values: &B::ValuePool,
        builder: &mut TimelineBuilder<'_, B::World>,
    ) -> Result<TrackFragment, CompileError<B>> {
        let resolver = self
            .fields
            .get::<FieldResolverBox<B>>(&cmd.field)
            .ok_or_else(|| {
                CompileError::UnknownField(cmd.field.clone())
            })?;

        resolver.build(cmd, self, values, builder)
    }
}

impl<B: SceneBackend> Default for SceneRegistry<B> {
    fn default() -> Self {
        Self::new()
    }
}
