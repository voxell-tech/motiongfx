// TODO: Write docs!

#![no_std]

extern crate alloc;

use bevy_app::prelude::*;
use bevy_asset::AsAssetId;
use bevy_ecs::component::Mutable;
use bevy_ecs::prelude::*;
#[cfg(feature = "transform")]
use bevy_transform::TransformSystem;

use crate::accessor::{Accessor, FieldAccessorRegistry};
use crate::field::Field;
use crate::pipeline::PipelineRegistry;

pub mod accessor;
pub mod action;
pub mod ease;
pub mod field;
pub mod interpolation;
pub mod pipeline;
pub mod sequence;
pub mod timeline;
pub mod track;

pub mod prelude {
    pub use crate::accessor::{Accessor, FieldAccessorRegistry};
    pub use crate::action::{ActionId, EaseFn, InterpFn};
    pub use crate::ease;
    pub use crate::field::{field, Field, UntypedField};
    pub use crate::interpolation::Interpolation;
    pub use crate::pipeline::{
        BakeCtx, PipelineKey, PipelineRegistry, SampleCtx,
    };
    pub use crate::register_fields;
    pub use crate::timeline::{Timeline, TimelineBuilder};
    pub use crate::FieldPathRegisterAppExt;
}

pub struct MotionGfxEnginePlugin;

impl Plugin for MotionGfxEnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FieldAccessorRegistry>()
            .init_resource::<PipelineRegistry>();

        app.configure_sets(
            PostUpdate,
            (
                MotionGfxSet::TargetTime,
                MotionGfxSet::QueueAction,
                #[cfg(not(feature = "transform"))]
                MotionGfxSet::Sample,
                #[cfg(feature = "transform")]
                MotionGfxSet::Sample
                    .before(TransformSystem::TransformPropagate),
                MotionGfxSet::CurrentTime,
            )
                .chain(),
        );
    }
}

/// Recursively register fields.
///
/// # Example
///
/// ```
/// use bevy_app::App;
/// use bevy_ecs::component::Component;
/// use motiongfx_engine::MotionGfxEnginePlugin;
/// use motiongfx_engine::prelude::*;
///
/// #[derive(Component, Default, Clone)]
/// struct Foo {
///     bar_x: Bar,
///     bar_y: Bar,
/// }
///
/// #[derive(Clone, Default)]
/// struct Bar {
///     cho_a: Cho,
///     cho_b: Cho,
/// }
///
/// #[derive(Clone, Default)]
/// struct Cho {
///     bo_c: Bo,
///     bo_d: Bo,
/// }
///
/// #[derive(Clone, Default)]
/// struct Bo(f32, u32);
///
/// let mut app = App::new();
/// app.add_plugins(MotionGfxEnginePlugin);
///
/// register_fields!(
///     app.register_component_field(),
///     Foo,
///     (
///         bar_x(cho_a(bo_c(0, 1), bo_d(0, 1))),
///         bar_y(cho_b(bo_c(0, 1), bo_d(0, 1))),
///     )
/// );
///
/// // Get accessor from the registry.
/// let accessor_registry =
///     app.world().resource::<FieldAccessorRegistry>();
///
/// let key = field!(<Foo>::bar_x::cho_a::bo_c::0).untyped();
/// let accessor =
///     accessor_registry.get::<Foo, f32>(&key).unwrap();
///
/// let mut foo = Foo::default();
///
/// assert_eq!((accessor.ref_fn)(&foo), &foo.bar_x.cho_a.bo_c.0,);
///
/// *(accessor.mut_fn)(&mut foo) = 2.0;
/// assert_eq!((accessor.ref_fn)(&foo), &2.0);
///
/// // Get pipeline from the registry.
/// let pipeline_registry =
///     app.world().resource::<PipelineRegistry>();
///
/// let key = PipelineKey::new::<Foo, f32>();
/// let pipeline = pipeline_registry.get(&key).unwrap();
/// ```
#[macro_export]
macro_rules! register_fields {
    (
        $app:ident.$reg_func:ident(),
        $root:ty $(, $($rest:tt)*)?
    ) => {
        $app.$reg_func(
            field!(<$root>),
            Accessor {
                ref_fn: |v| v,
                mut_fn: |v| v,
            }
        );

        register_fields!(
            @fields $app.$reg_func::<$root>,
            $root, []
            $(, $($rest)*)?
        );
    };

    (
        $app:ident.$reg_func:ident::<$source:ty>(),
        $root:ty $(, $($rest:tt)*)?
    ) => {
        $app.$reg_func::<$source, _>(
            field!(<$root>),
            Accessor {
                ref_fn: |v| v,
                mut_fn: |v| v,
            }
        );

        register_fields!(
            @fields $app.$reg_func::<$source>, $root, []
            $(, $($rest)*)?
        );
    };

    // Recursively register all the nested fields!
    (
        @fields $app:ident.$reg_func:ident::<$source:ty>,
        $root:ty, [$(::$path:tt)*],
        (
            $field:tt $(( $($sub_field:tt)+ ))?
            $(,$($rest:tt)*)?
        )
    ) => {
        // Register the current field.
        // (translation(x, y, z), rotation, scale) => translation
        $app.$reg_func::<$source, _>(
            field!(<$root>$(::$path)*::$field),
            Accessor {
                ref_fn: |v| &v$(.$path)*.$field,
                mut_fn: |v| &mut v$(.$path)*.$field,
            },
        );

        // Register sub fields.
        // (translation(x, y, z), rotation, scale) => (x, y, z)
        register_fields!(
            @fields $app.$reg_func::<$source>,
            $root, [$(::$path)*::$field],
            $(( $($sub_field)+ ))?
        );

        // Register the rest of the fields.
        // (translation(x, y, z), rotation, scale) => (rotation, scale)
        register_fields!(
            @fields $app.$reg_func::<$source>,
            $root, [$(::$path)*],
            $(( $($rest)* ))?
        );
    };

    // There are no fields left!
    (
        @fields $app:ident.$reg_func:ident::<$source:ty>,
        $root:ty, [$(::$path:tt)*]
        $(,)? $(,())?
    ) => {};
}

pub trait FieldPathRegisterAppExt {
    fn register_component_field<S, T>(
        &mut self,
        field: Field<S, T>,
        accessor: Accessor<S, T>,
    ) -> &mut Self
    where
        S: Component<Mutability = Mutable>,
        T: Clone + ThreadSafe;

    #[cfg(feature = "asset")]
    fn register_asset_field<S, T>(
        &mut self,
        field: Field<S::Asset, T>,
        accessor: Accessor<S::Asset, T>,
    ) -> &mut Self
    where
        S: AsAssetId,
        T: Clone + ThreadSafe;
}

impl FieldPathRegisterAppExt for App {
    fn register_component_field<S, T>(
        &mut self,
        field: Field<S, T>,
        accessor: Accessor<S, T>,
    ) -> &mut Self
    where
        S: Component<Mutability = Mutable>,
        T: Clone + ThreadSafe,
    {
        self.world_mut()
            .resource_mut::<FieldAccessorRegistry>()
            .register(field.untyped(), accessor);

        self.world_mut()
            .resource_mut::<PipelineRegistry>()
            .register_component::<S, T>();

        self
    }

    #[cfg(feature = "asset")]
    fn register_asset_field<S, T>(
        &mut self,
        field: Field<S::Asset, T>,
        accessor: Accessor<S::Asset, T>,
    ) -> &mut Self
    where
        S: AsAssetId,
        T: Clone + ThreadSafe,
    {
        self.world_mut()
            .resource_mut::<FieldAccessorRegistry>()
            .register(field.untyped(), accessor);

        self.world_mut()
            .resource_mut::<PipelineRegistry>()
            .register_asset::<S, T>();

        self
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MotionGfxSet {
    /// Advance the target time in the [`SequenceController`].
    TargetTime,
    /// Mark actions that are affected by the `target_time`
    /// change in [`SequenceController`].
    QueueAction,
    /// Sample keyframes and applies the value.
    /// This happens before [`TransformSystem::TransformPropagate`].
    Sample,
    /// Advance the current time in the [`SequenceController`].
    CurrentTime,
}

/// Auto trait for types that implements [`Send`] + [`Sync`] + `'static`.
pub trait ThreadSafe: Send + Sync + 'static {}

impl<T> ThreadSafe for T where T: Send + Sync + 'static {}
