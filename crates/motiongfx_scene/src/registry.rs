//! The reconstruction boundary: maps scene names to typed runtime
//! closures. Filled by the backend at startup; see [`SceneRegistry`].

use core::any::{Any, TypeId};
use core::marker::PhantomData;

use alloc::boxed::Box;

use hashbrown::HashMap;

use motiongfx::ThreadSafe;
use motiongfx::action::{Action, EaseFn, InterpFn};
use motiongfx::field_path::field::UntypedField;
use motiongfx::prelude::*;
use motiongfx::registry::Registry;
use motiongfx::subject::SubjectId;
use motiongfx::world::SubjectSource;

use crate::block::ActionCmd;
use crate::error::CompileError;
use crate::refs::{EaseRef, FieldRef, InterpRef, OpRef, TypeName};

/// Resolves one `S`/`T`-typed field into a [`TrackFragment`], stored by
/// [`FieldRef`]. The action itself (looked up by `T` alone, not `S`; see
/// [`SceneRegistry::build_action`] and its `action_resolvers` map) doesn't
/// belong here - only the field-accessor step needs `S`.
trait FieldResolver<Id, V, W> {
    fn build(
        &self,
        cmd: &ActionCmd<Id, V>,
        registry: &SceneRegistry<Id, V, W>,
        builder: &mut TimelineBuilder<'_, W>,
    ) -> Result<TrackFragment, CompileError<Id>>;
}

struct ConcreteFieldResolver<Id, V, W, S, T> {
    _marker: PhantomData<(Id, V, W, S, T)>,
}

impl<Id, V, W, S, T> FieldResolver<Id, V, W>
    for ConcreteFieldResolver<Id, V, W, S, T>
where
    W: SubjectSource<Id, S> + 'static,
    Id: SubjectId + 'static,
    S: 'static,
    T: ThreadSafe + Clone + 'static,
    V: 'static,
{
    fn build(
        &self,
        cmd: &ActionCmd<Id, V>,
        registry: &SceneRegistry<Id, V, W>,
        builder: &mut TimelineBuilder<'_, W>,
    ) -> Result<TrackFragment, CompileError<Id>> {
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
        let action =
            registry.build_action::<T>(&cmd.op, &cmd.value)?;

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

type BuildAction<V, T> = Box<dyn Fn(&V) -> Box<dyn Action<T>>>;

/// Keyed by `(TypeId::of::<T>(), OpRef)`: an op's identity is the value
/// type it acts on plus its name, never the field's owning type.
type ActionResolverMap = HashMap<(TypeId, OpRef), Box<dyn Any>>;

type FieldResolverMap<Id, V, W> =
    HashMap<FieldRef, Box<dyn FieldResolver<Id, V, W>>>;

type FieldRegistrarMap =
    HashMap<FieldRef, Box<dyn Fn(&mut Registry)>>;

/// The bridge between scene names and runtime closures.
///
/// `Id`/`V` match the scene's type parameters; `W` is the runtime's
/// world type. Fill via [`Self::register_field`], [`Self::register_op`],
/// and optionally [`Self::register_ease`]/[`Self::register_interp`].
pub struct SceneRegistry<Id, V, W> {
    action_resolvers: ActionResolverMap,
    field_map: HashMap<FieldRef, UntypedField>,
    field_resolvers: FieldResolverMap<Id, V, W>,
    field_registrars: FieldRegistrarMap,
    eases: HashMap<EaseRef, EaseFn>,
    interps: HashMap<InterpRef, Box<dyn Any>>,
    _marker: PhantomData<(Id, V, W)>,
}

impl<Id, V, W> SceneRegistry<Id, V, W> {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            action_resolvers: HashMap::new(),
            field_map: HashMap::new(),
            field_resolvers: HashMap::new(),
            field_registrars: HashMap::new(),
            eases: HashMap::new(),
            interps: HashMap::new(),
            _marker: PhantomData,
        }
    }

    /// Register a field mapping.
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
        W: SubjectSource<Id, S> + 'static,
        Id: SubjectId + 'static,
        S: 'static,
        T: ThreadSafe + Clone,
        V: 'static,
    {
        let field_ref = FieldRef {
            type_name,
            path: path.into(),
        };
        let untyped = field_acc.field.untyped();
        self.field_map.insert(field_ref.clone(), untyped);
        self.field_resolvers.insert(
            field_ref.clone(),
            Box::new(ConcreteFieldResolver::<Id, V, W, S, T> {
                _marker: PhantomData,
            }),
        );

        // `FieldAccessor`'s derived `Copy` needs `S: Copy, T: Copy`, but
        // `Field`/`Accessor` alone are unconditionally `Copy` - capture
        // those instead to keep this closure `Fn`, not `FnOnce`.
        let field = field_acc.field;
        let accessor = field_acc.accessor;
        self.field_registrars.insert(
            field_ref,
            Box::new(move |runtime: &mut Registry| {
                runtime.register::<W, Id, S, T>(FieldAccessor::new(
                    field, accessor,
                ));
            }),
        );
    }

    /// Install every registered field's accessor into `runtime_registry`.
    pub(crate) fn install_accessors(
        &self,
        runtime_registry: &mut Registry,
    ) {
        for registrar in self.field_registrars.values() {
            registrar(runtime_registry);
        }
    }

    /// Register an op by name for a value type `T`.
    ///
    /// Keyed by `T` alone (via [`TypeId`]), not by any field's owning
    /// type `S`: the same `"to"`/`"by"` registered for `T = f32` covers
    /// every field of every `S` whose value type is `f32`. `build_action`
    /// receives a reference to the scene's opaque value `V` and returns
    /// a boxed action closure.
    pub fn register_op<T, F>(&mut self, op: OpRef, build_action: F)
    where
        T: ThreadSafe + Clone + 'static,
        V: 'static,
        F: Fn(&V) -> Box<dyn Action<T>> + 'static,
    {
        let build_action: BuildAction<V, T> = Box::new(build_action);
        self.action_resolvers
            .insert((TypeId::of::<T>(), op), Box::new(build_action));
    }

    /// Register an easing function by name.
    pub fn register_ease(&mut self, name: EaseRef, ease: EaseFn) {
        self.eases.insert(name, ease);
    }

    /// Register an interpolation function by name.
    pub fn register_interp<T>(
        &mut self,
        name: InterpRef,
        interp: InterpFn<T>,
    ) where
        T: 'static,
    {
        self.interps.insert(name, Box::new(interp));
    }

    pub(crate) fn resolve_field(
        &self,
        field_ref: &FieldRef,
    ) -> Result<UntypedField, CompileError<Id>> {
        self.field_map.get(field_ref).copied().ok_or_else(|| {
            CompileError::UnknownField(field_ref.clone())
        })
    }

    pub(crate) fn resolve_ease(
        &self,
        ease: &Option<EaseRef>,
    ) -> Option<EaseFn> {
        ease.as_ref().and_then(|name| self.eases.get(name).copied())
    }

    /// `None` falls back to step interpolation; an unregistered
    /// `Some(name)` is [`CompileError::UnknownInterp`].
    pub(crate) fn resolve_interp<T>(
        &self,
        interp: &Option<InterpRef>,
    ) -> Result<InterpFn<T>, CompileError<Id>>
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
                .get(name)
                .and_then(|f| f.downcast_ref::<InterpFn<T>>())
                .copied()
                .ok_or_else(|| {
                    CompileError::UnknownInterp(name.clone())
                }),
        }
    }

    /// Build the action for `op` under a concrete `T`, or
    /// [`CompileError::UnknownOp`] if `op` isn't registered for this `T`.
    pub(crate) fn build_action<T>(
        &self,
        op: &OpRef,
        value: &V,
    ) -> Result<Box<dyn Action<T>>, CompileError<Id>>
    where
        T: 'static,
        V: 'static,
    {
        self.action_resolvers
            .get(&(TypeId::of::<T>(), op.clone()))
            .and_then(|f| f.downcast_ref::<BuildAction<V, T>>())
            .map(|build_action| build_action(value))
            .ok_or_else(|| {
                CompileError::UnknownOp(
                    core::any::type_name::<T>(),
                    op.clone(),
                )
            })
    }

    pub(crate) fn resolve_op(
        &self,
        cmd: &ActionCmd<Id, V>,
        builder: &mut TimelineBuilder<'_, W>,
    ) -> Result<TrackFragment, CompileError<Id>> {
        let resolver =
            self.field_resolvers.get(&cmd.field).ok_or_else(
                || CompileError::UnknownField(cmd.field.clone()),
            )?;

        resolver.build(cmd, self, builder)
    }
}

impl<Id, V, W> Default for SceneRegistry<Id, V, W> {
    fn default() -> Self {
        Self::new()
    }
}
