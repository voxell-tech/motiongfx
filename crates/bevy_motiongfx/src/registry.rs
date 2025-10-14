use bevy_app::prelude::*;
#[cfg(feature = "asset")]
use bevy_asset::Asset;
use bevy_ecs::component::Mutable;
use bevy_ecs::prelude::*;
use motiongfx::prelude::*;

use crate::pipeline::{PipelineRegistryExt, WorldPipelineRegistry};

// TODO: Move purely the recursive logic back to motiongfx and keep
// the registration logic here.

/// Recursively register fields.
///
/// # Example
///
/// ```
/// use bevy_ecs::entity::Entity;
/// use bevy_app::App;
/// use bevy_ecs::component::Component;
/// use bevy_motiongfx::BevyMotionGfxPlugin;
/// use bevy_motiongfx::prelude::*;
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
/// app.add_plugins(BevyMotionGfxPlugin);
///
/// let a = &mut app;
/// register_fields!(
///     a.register_component_field(),
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
///     app.world().resource::<WorldPipelineRegistry>();
///
/// let key = PipelineKey::new::<Entity, Foo, f32>();
/// let pipeline = pipeline_registry.get(&key).unwrap();
/// ```
#[macro_export]
macro_rules! register_fields {
    (
        $app:ident.$reg_func:ident(),
        $root:ty $(, $($rest:tt)*)?
    ) => {
        register_fields!(
            $app.$reg_func::<$root>(),
            $root $(, $($rest)*)?
        )
    };

    (
        $app:ident.$reg_func:ident::<$source:ty>(),
        $root:ty $(, $($rest:tt)*)?
    ) => {
        $crate::registry::FieldPathRegisterAppExt
        ::$reg_func::<$source, _>(
            $app,
            ::motiongfx::field::field!(<$root>),
            ::motiongfx::accessor::Accessor {
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
        $crate::registry::FieldPathRegisterAppExt
        ::$reg_func::<$source, _>(
            $app,
            motiongfx::field::field!(<$root>$(::$path)*::$field),
            ::motiongfx::accessor::Accessor {
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
        field: Field<S, T>,
        accessor: Accessor<S, T>,
    ) -> &mut Self
    where
        S: Asset,
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
            .resource_mut::<WorldPipelineRegistry>()
            .register_component::<S, T>();

        self
    }

    #[cfg(feature = "asset")]
    fn register_asset_field<S, T>(
        &mut self,
        field: Field<S, T>,
        accessor: Accessor<S, T>,
    ) -> &mut Self
    where
        S: Asset,
        T: Clone + ThreadSafe,
    {
        self.world_mut()
            .resource_mut::<FieldAccessorRegistry>()
            .register(field.untyped(), accessor);

        self.world_mut()
            .resource_mut::<WorldPipelineRegistry>()
            .register_asset::<S, T>();

        self
    }
}
