use std::any::TypeId;
use std::marker::PhantomData;

use bevy::ecs::component::{ComponentHooks, Immutable, StorageType};
use bevy::platform::collections::HashMap;
use bevy::platform::hash::Hashed;
use bevy::prelude::*;

pub(super) struct FieldPlugin;

impl Plugin for FieldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FieldMap>();
    }
}

pub trait RegisterFieldAppExt {
    fn register_field<Source, Target>(
        &mut self,
        field_bundle: FieldBundle<Source, Target>,
    ) -> &mut Self
    where
        Source: ThreadSafe,
        Target: ThreadSafe;
}

impl RegisterFieldAppExt for App {
    /// Spawns the [`FieldBundle`] and stores it in the
    /// [`FieldMap`] resource.
    fn register_field<Source, Target>(
        &mut self,
        field_bundle: FieldBundle<Source, Target>,
    ) -> &mut Self
    where
        Source: ThreadSafe,
        Target: ThreadSafe,
    {
        let field_map = self.world().resource::<FieldMap>();
        let field_hash = field_bundle.field.to_hash();

        if field_map.contains_key(&field_hash) {
            error!(
                "(<{}> {}) was registered more than once!",
                core::any::type_name::<Source>(),
                field_bundle.field.field_path
            );
            return self;
        }

        let entity =
            self.world_mut().commands().spawn(field_bundle).id();

        self.world_mut()
            .resource_mut::<FieldMap>()
            .0
            .insert(field_hash, entity);

        self
    }
}

/// Maps a [`FieldHash`] to an entity with [`FieldAccessor`].
#[derive(Resource, Deref, Default, Debug)]
pub struct FieldMap(HashMap<FieldHash, Entity>);

#[derive(Bundle, Debug)]
pub struct FieldBundle<Source, Target>
where
    Source: ThreadSafe,
    Target: ThreadSafe,
{
    pub accessor: FieldAccessor<Source, Target>,
    pub field: Field<Source, Target>,
}

impl<Source, Target> Clone for FieldBundle<Source, Target>
where
    Source: ThreadSafe,
    Target: ThreadSafe,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Source, Target> Copy for FieldBundle<Source, Target>
where
    Source: ThreadSafe,
    Target: ThreadSafe,
{
}

#[macro_export]
macro_rules! field_bundle {
    (<$source:ty>$(::$field:tt)*) => {
        $crate::field::FieldBundle {
            accessor: $crate::field::accessor!(<$source>$(::$field)*),
            field: $crate::field::field!(<$source>$(::$field)*),
        }
    };
}
pub use field_bundle;

/// A statically typed field path with the `Source`
/// and `Target` type stored in the generics of the struct.
///
/// Inserting this component to an entity will also create
/// an equivalent [`FieldHash`] component to that entity via
/// the `on_insert` hook.
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Field<Source, Target> {
    /// The path of the field in the source.
    /// (e.g. `Transform::translation::x` will have
    /// a field path of ":: translation :: x").
    ///
    /// You can achieved this using [`stringify!`].
    pub field_path: &'static str,
    _marker: PhantomData<(Source, Target)>,
}

impl<Source, Target> Field<Source, Target> {
    pub const fn new(field_path: &'static str) -> Self {
        Self {
            field_path,
            _marker: PhantomData,
        }
    }
}

impl<Source, Target> Field<Source, Target>
where
    Source: 'static,
{
    pub fn to_hash(&self) -> FieldHash {
        FieldHash::new::<Source>(self.field_path)
    }
}

impl<Source, Target> Component for Field<Source, Target>
where
    Source: ThreadSafe,
    Target: ThreadSafe,
{
    const STORAGE_TYPE: StorageType = StorageType::Table;

    type Mutability = Immutable;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_insert(|mut world, context| {
            // SAFETY: Hook should only trigger after the component is inserted.
            let field = world.get::<Self>(context.entity).unwrap();
            let field_hash = field.to_hash();

            // Add FieldHash that corresponds to the Field.
            world
                .commands()
                .entity(context.entity)
                .insert(field_hash);
        });
    }
}

impl<Source, Target> Copy for Field<Source, Target> {}

impl<Source, Target> Clone for Field<Source, Target> {
    fn clone(&self) -> Self {
        *self
    }
}

pub struct FieldBuilder<Source, Target> {
    /// See [`Field::field_path`]
    field_path: &'static str,
    /// A marker function to identify the validity of the path
    /// in the [`field!`] macro.
    _field_marker: fn(source: Source) -> Target,
}

impl<Source, Target> FieldBuilder<Source, Target> {
    pub const fn new(
        _field_marker: fn(source: Source) -> Target,
        field_path: &'static str,
    ) -> Self {
        Self {
            field_path,
            _field_marker,
        }
    }

    pub const fn build(self) -> Field<Source, Target> {
        Field {
            field_path: self.field_path,
            _marker: PhantomData,
        }
    }
}

/// Creates a [`Field`] with type and path safety.
#[macro_export]
macro_rules! field {
    (<$source:ty>$(::$field:tt)*) => {
        {
            let builder = $crate::field::FieldBuilder::new(
                |source: $source| source$(.$field)*,
                stringify!($(::$field)*),
            );

            builder.build()
        }
    };
}
pub use field;

/// A pre-hashed (source-type-id, field-path) pair,
/// uniquely identifies a source's field path.
///
/// ### source-type-id
///
/// The [`TypeId`] of the source.
///
/// ### field-path
///
/// See [`Field::field_path`]
///
/// You can achieved this using [`stringify!`].
#[derive(
    Component,
    Deref,
    DerefMut,
    Debug,
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
)]
pub struct FieldHash(pub Hashed<(TypeId, &'static str)>);

impl FieldHash {
    /// Creates a new field hash that will be used to identify
    /// the field inside a specific component.
    ///
    /// This supports nested fields too (e.g. `Transform::translation::x`).
    ///
    /// # Validation
    ///
    /// The field path used must be valid.
    /// To prevent using the wrong path, use [`field_hash!`].
    pub fn new<Source: 'static>(field_path: &'static str) -> Self {
        Self(Hashed::new((TypeId::of::<Source>(), field_path)))
    }
}

impl<Source, Target> From<Field<Source, Target>> for FieldHash
where
    Source: 'static,
{
    fn from(value: Field<Source, Target>) -> Self {
        value.to_hash()
    }
}

/// Creates a [`FieldHash`] without type safety.
///
/// Use [`field!`] for the type safe version.
#[macro_export]
macro_rules! field_hash_raw {
    (<$source:ty>$(::$field:tt)*) => {
        $crate::field::FieldHash::new::<$source>(stringify!($(::$field)*))
    };
}
pub use field_hash_raw;

/// Creates a [`FieldHash`] with type safety.
///
/// Use [`field_hash_raw!`] for the non type safe version.
#[macro_export]
macro_rules! field_hash {
    (<$source:ty>$(::$field:tt)+) => {
        {
            // This is just to make sure that the rust compiler
            // catches the error if a wrong field is entered.
            #[cfg(debug_assertions)]
            let _ = |ty: $source| ty$(.$field)+;

            $crate::field::field_hash_raw!(<$source>$(::$field)+)
        }
    };
    (<$source:ty>) => {
        $crate::field::field_hash_raw!(<$source>)
    };
}
pub use field_hash;

/// A wrapper of [`FieldRefFn`] and [`FieldMutFn`]
/// for attaching to an entity as a component.
///
/// Allows access to the `Target` type from
/// the `Source` type both immutably and mutably.
#[derive(Component, Reflect, Debug, PartialEq, Eq)]
#[reflect(Component)]
#[component(immutable)]
pub struct FieldAccessor<Source, Target> {
    pub field_ref: FieldRefFn<Source, Target>,
    pub field_mut: FieldMutFn<Source, Target>,
}

impl<Source, Target> FieldAccessor<Source, Target> {
    /// Get a immutable reference of the `Target` from the `Source`.
    pub fn get_ref<'a>(&self, source: &'a Source) -> &'a Target {
        (self.field_ref)(source)
    }

    /// Get a mutable reference of the `Target` from the `Source`.
    pub fn get_mut<'a>(
        &self,
        source: &'a mut Source,
    ) -> &'a mut Target {
        (self.field_mut)(source)
    }
}

impl<Source, Target> Copy for FieldAccessor<Source, Target> {}

impl<Source, Target> Clone for FieldAccessor<Source, Target> {
    fn clone(&self) -> Self {
        *self
    }
}

#[macro_export]
macro_rules! accessor {
    (<$source:ty>$(::$field:tt)+) => {
        $crate::field::FieldAccessor {
            field_ref: |source: &$source| &source$(.$field)+,
            field_mut: |source: &mut $source| &mut source$(.$field)+,
        }
    };
    (<$source:ty>) => {
        $crate::field::FieldAccessor {
            field_ref: |source: &$source| source,
            field_mut: |source: &mut $source| source,
        }
    };
}
pub use accessor;

use crate::ThreadSafe;

/// Function for getting a immutable reference of `Target` from `Source`.
/// The `Target` type can be similar to `Source` as well.
pub type FieldRefFn<Source, Target> = fn(source: &Source) -> &Target;

/// Function for getting a mutable reference of `Target` from `Source`.
/// The `Target` type can be similar to `Source` as well.
pub type FieldMutFn<Source, Target> =
    fn(source: &mut Source) -> &mut Target;

#[cfg(test)]
mod test {
    use super::*;

    #[derive(PartialEq, Debug, Clone)]
    struct Index(u32);

    #[derive(PartialEq, Debug, Clone)]
    struct Name(String);

    #[derive(PartialEq, Debug, Clone)]
    struct NestedName {
        name: Name,
    }

    impl NestedName {
        pub fn new(name: &str) -> Self {
            Self {
                name: Name(name.to_string()),
            }
        }
    }

    #[test]
    fn test_field() {
        let field_same = field!(<Index>);
        assert_eq!(field_same.field_path, "");
        assert_eq!(field_same.to_hash(), field_hash!(<Index>));

        let field_inner = field!(<Index>::0);
        assert_eq!(field_inner.field_path, stringify!(::0));
        assert_eq!(field_inner.to_hash(), field_hash!(<Index>::0));
    }

    #[test]
    fn test_nested_field() {
        let field = field!(<NestedName>::name::0);
        assert_eq!(field.field_path, stringify!(::name::0));
        assert_eq!(
            field.to_hash(),
            field_hash!(<NestedName>::name::0)
        );
    }

    #[test]
    fn test_field_hash() {
        let field_hash_same = field_hash!(<Index>);
        assert_eq!(field_hash_same, FieldHash::new::<Index>(""));

        let field_hash_inner = field_hash!(<Index>::0);
        assert_eq!(
            field_hash_inner,
            FieldHash::new::<Index>(stringify!(::0))
        );

        // Both should not be the same.
        assert_ne!(field_hash_same, field_hash_inner);
    }

    #[test]
    fn test_nested_field_hash() {
        let field_hash = field_hash!(<NestedName>::name::0);
        assert_eq!(
            field_hash,
            FieldHash::new::<NestedName>(stringify!(::name::0))
        );
    }

    #[test]
    fn test_accessor() {
        let mut value = Index(6);

        let accessor = accessor!(<Index>);

        assert_eq!(accessor.get_ref(&value).0, 6);

        accessor.get_mut(&mut value).0 += 1;
        assert_eq!(value.0, 7);
    }

    #[test]
    fn test_nested_accessor() {
        let mut value = NestedName::new("bob");

        let accessor = accessor!(<NestedName>::name::0);

        assert_eq!(&value.name.0, accessor.get_ref(&value));

        accessor.get_mut(&mut value).push_str(" likes sweet.");
        assert_eq!(value.name.0, "bob likes sweet");
    }

    #[test]
    fn test_field_bundle() {
        let FieldBundle { field, accessor } =
            field_bundle!(<NestedName>::name::0);

        assert_eq!(field, field!(<NestedName>::name::0));
        assert_eq!(accessor, accessor!(<NestedName>::name::0));
    }
}
