//! The reconstruction boundary: maps scene names to typed runtime
//! closures. Filled by the backend at startup; see [`SceneRegistry`].

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

/// A monomorphized op builder, stored by `(TypeName, OpRef)`.
trait TypedOpBuilder<Id, V, W> {
    fn build(
        &self,
        cmd: &ActionCmd<Id, V>,
        registry: &SceneRegistry<Id, V, W>,
        builder: &mut TimelineBuilder<'_, W>,
    ) -> Result<TrackFragment, CompileError<Id>>;
}

struct ConcreteOpBuilder<Id, V, W, S, T, F> {
    f: F,
    _marker: PhantomData<(Id, V, W, S, T)>,
}

impl<Id, V, W, S, T, F> TypedOpBuilder<Id, V, W>
    for ConcreteOpBuilder<Id, V, W, S, T, F>
where
    W: SubjectSource<Id, S> + 'static,
    Id: SubjectId + 'static,
    S: 'static,
    T: ThreadSafe + Clone + 'static,
    V: 'static,
    F: Fn(&V) -> Box<dyn Action<T>>,
{
    fn build(
        &self,
        cmd: &ActionCmd<Id, V>,
        registry: &SceneRegistry<Id, V, W>,
        builder: &mut TimelineBuilder<'_, W>,
    ) -> Result<TrackFragment, CompileError<Id>> {
        let field = registry.resolve_field(&cmd.field)?;

        // Verify type match and get the typed accessor.
        let accessor = builder
            .registry()
            .accessor
            .get::<S, T>(&field)
            .ok_or_else(|| CompileError::TypeMismatch {
                type_name: core::any::type_name::<T>(),
                field: cmd.field.clone(),
            })?;

        // Reconstruct the typed Field and FieldAccessor.
        let typed_field = field.typed::<S, T>().ok_or_else(|| {
            CompileError::TypeMismatch {
                type_name: core::any::type_name::<T>(),
                field: cmd.field.clone(),
            }
        })?;

        let field_acc = FieldAccessor::new(typed_field, accessor);
        let action = (self.f)(&cmd.value);

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

type OpBuilderMap<Id, V, W> =
    HashMap<(TypeName, OpRef), Box<dyn TypedOpBuilder<Id, V, W>>>;

type FieldRegistrarMap =
    HashMap<FieldRef, Box<dyn Fn(&mut Registry)>>;

/// The bridge between scene names and runtime closures.
///
/// `Id`/`V` match the scene's type parameters; `W` is the runtime's
/// world type. Fill via [`register_field`](Self::register_field),
/// [`register_op`](Self::register_op), and optionally
/// [`register_ease`](Self::register_ease)/[`register_interp`](Self::register_interp).
pub struct SceneRegistry<Id, V, W> {
    op_builders: OpBuilderMap<Id, V, W>,
    field_map: HashMap<FieldRef, UntypedField>,
    field_registrars: FieldRegistrarMap,
    eases: HashMap<EaseRef, EaseFn>,
    interps: HashMap<InterpRef, Box<dyn core::any::Any>>,
    _marker: PhantomData<(Id, V, W)>,
}

impl<Id, V, W> SceneRegistry<Id, V, W> {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            op_builders: HashMap::new(),
            field_map: HashMap::new(),
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
    {
        let field_ref = FieldRef {
            type_name,
            path: path.into(),
        };
        let untyped = field_acc.field.untyped();
        self.field_map.insert(field_ref.clone(), untyped);

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

    /// Register an action op builder.
    ///
    /// `f` receives a reference to the scene's opaque value `V` and
    /// returns a boxed action closure.
    pub fn register_op<S, T, F>(
        &mut self,
        type_name: TypeName,
        op: OpRef,
        f: F,
    ) where
        W: SubjectSource<Id, S> + 'static,
        Id: SubjectId + 'static,
        S: 'static,
        T: ThreadSafe + Clone + 'static,
        V: 'static,
        F: Fn(&V) -> Box<dyn Action<T>> + 'static,
    {
        self.op_builders.insert(
            (type_name, op),
            Box::new(ConcreteOpBuilder::<Id, V, W, S, T, F> {
                f,
                _marker: PhantomData,
            }),
        );
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

    pub(crate) fn resolve_op(
        &self,
        cmd: &ActionCmd<Id, V>,
        builder: &mut TimelineBuilder<'_, W>,
    ) -> Result<TrackFragment, CompileError<Id>> {
        let key = (cmd.field.type_name.clone(), cmd.op.clone());
        let op_builder =
            self.op_builders.get(&key).ok_or_else(|| {
                CompileError::UnknownOp(
                    cmd.field.type_name.clone(),
                    cmd.op.clone(),
                )
            })?;

        op_builder.build(cmd, self, builder)
    }
}

impl<Id, V, W> Default for SceneRegistry<Id, V, W> {
    fn default() -> Self {
        Self::new()
    }
}
