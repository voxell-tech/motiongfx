//! Provides abstractions for working with typed fields in a
//! type-erased, yet type-safe manner.
//!
//! A [`Field`] represents a way to identify and a field path within a
//! data structure.
//!
//! This module is fully independent and can be used in isolation.
//! It is intended to serve as a flexible building block for systems
//! that need to store, compare, or retrieve fields dynamically.

use core::any::TypeId;
use core::marker::PhantomData;

use bevy::prelude::*;

/// A statically typed field path from a source type `S` to a target
/// type `T`.
///
/// It uniquely identifies a target field path within a source `struct`
/// through the `field_path`. The type parameters encode both the
/// source type `S` and the resolved target type `T`.
///
/// A `Field` can also be created at compile time, allowing us to
/// create `const` or `static` fields.
///
/// # Validation
///
/// The [`field!`] macro ensures that both the path and type used are
/// valid. Constructing `Field` manually may result in mismatches.
///
/// # Example
/// ```
/// use motiongfx_engine::field::{Field, field};
///
/// struct Player {
///     name: String,
///     age: u32,
/// }
///
/// let name: Field<Player, String> = field!(<Player>::name);
/// let age: Field<Player, u32> = field!(<Player>::age);
///
/// assert_ne!(name.untyped(), age.untyped());
/// ```
#[derive(Component, Debug, Hash, PartialEq, Eq)]
pub struct Field<S, T> {
    /// The path of the target field in the source.
    /// (e.g. `Transform::translation::x` will have
    /// a field path of `"::translation::x"`).
    ///
    /// This can be achieved using [`stringify_field!`].
    field_path: &'static str,
    _marker: PhantomData<(S, T)>,
}

impl<S, T> Field<S, T> {
    /// A field with a placeholder field path. This does not correspond
    /// to a vaild field path!
    pub const PLACEHOLDER: Self = Self::new("$");

    /// Construct a new [`Field`] from a raw field path string.
    ///
    /// The field path can be constructed via the [`stringify_field!`]
    /// macro for convenience.
    ///
    /// Prefer the [`field!`] macro for type safety!
    pub const fn new(field_path: &'static str) -> Self {
        Self {
            field_path,
            _marker: PhantomData,
        }
    }

    /// Returns the raw field path string.
    ///
    /// Example: `Transform::translation::x` will have a field path
    /// of `"::translation::x"`.
    ///
    /// This can be achieved using [`stringify_field!`].
    pub fn field_path(&self) -> &'static str {
        self.field_path
    }
}

impl<S, T> Field<S, T>
where
    S: 'static,
    T: 'static,
{
    /// Converts into a [`UntypedField`] type.
    pub fn untyped(&self) -> UntypedField {
        UntypedField::new::<S, T>(self.field_path)
    }
}

impl<S, T> Copy for Field<S, T> {}

impl<Source, Target> Clone for Field<Source, Target> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Builder used internally by the [`field!`] macro to construct
/// [`Field`]s.
///
/// Ensures type correctness by using a marker function signature.
pub struct _FieldBuilder<S, T> {
    /// A marker function to identify the validity of the path
    /// in the [`field!`] macro.
    field_marker: fn(source: S) -> T,
    /// See [`Field::field_path`]
    field_path: &'static str,
}

impl<S, T> _FieldBuilder<S, T> {
    #[inline]
    pub const fn new(
        field_marker: fn(source: S) -> T,
        field_path: &'static str,
    ) -> Self {
        Self {
            field_path,
            field_marker,
        }
    }

    pub const fn build(self) -> Field<S, T> {
        Field {
            field_path: self.field_path,
            _marker: PhantomData,
        }
    }
}

/// Creates a [`Field`] with path and type safety.
///
/// # Example
///
/// ```
/// use motiongfx_engine::field::{Field, field};
///
/// struct Player {
///     name: String,
///     age: u32,
/// }
///
/// let player_name: Field<Player, String> = field!(<Player>::name);
/// let player_age: Field<Player, u32> = field!(<Player>::age);
///
/// assert_ne!(player_name.untyped(), player_age.untyped());
/// ```
#[macro_export]
macro_rules! field {
    (<$source:ty>$(::$field:tt)*) => {
        {
            let builder = $crate::field::_FieldBuilder::new(
                |source: $source| source$(.$field)*,
                $crate::stringify_field!($(::$field)*),
            );

            builder.build()
        }
    };
}
pub use field;

/// A type-erased version of [`Field`]. It uniquely identifies a
/// target field path within a source `struct`.
///
/// # Validation
///
/// The field path used must be valid. To prevent using the wrong
/// path, use the [`field_hash!`] macro. Alternatively, you can
/// also create one via [`Field::untyped`].
#[derive(
    Component,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
)]
pub struct UntypedField {
    /// The [`TypeId`] of the source type.
    source_id: TypeId,
    /// The [`TypeId`] of the target type.
    target_id: TypeId,
    /// See [`Field::field_path`].
    field_path: &'static str,
}

impl UntypedField {
    pub fn new<Source: 'static, Target: 'static>(
        field_path: &'static str,
    ) -> Self {
        Self {
            source_id: TypeId::of::<Source>(),
            target_id: TypeId::of::<Target>(),
            field_path,
        }
    }

    pub fn placeholder() -> Self {
        Self::placeholder_with_path("$")
    }

    pub fn placeholder_with_path(field_path: &'static str) -> Self {
        Self::new::<(), ()>(field_path)
    }

    /// Get the [`TypeId`] of the source type.
    pub fn source_id(&self) -> TypeId {
        self.source_id
    }

    /// Get the [`TypeId`] of the target type.
    pub fn target_id(&self) -> TypeId {
        self.target_id
    }

    /// See [`Field::field_path`].
    pub fn field_path(&self) -> &'static str {
        self.field_path
    }

    pub fn typed<Source: 'static, Target>(
        self,
    ) -> Field<Source, Target> {
        assert_eq!(TypeId::of::<Source>(), self.source_id);
        self.typed_unchecked()
    }

    pub fn typed_unchecked<S: 'static, T>(self) -> Field<S, T> {
        Field::new(self.field_path)
    }
}

impl<S, T> From<Field<S, T>> for UntypedField
where
    S: 'static,
    T: 'static,
{
    fn from(field: Field<S, T>) -> Self {
        field.untyped()
    }
}

impl<S, T> From<&Field<S, T>> for UntypedField
where
    S: 'static,
    T: 'static,
{
    fn from(field: &Field<S, T>) -> Self {
        field.untyped()
    }
}

#[macro_export]
macro_rules! stringify_field {
    ($(::$field:tt)*) => {
        concat!($("::", stringify!($field),)*)
    };
}
pub use stringify_field;

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
        let field = field!(<Index>);
        assert_eq!(field.field_path, "");

        let field = field!(<Index>::0);
        assert_eq!(field.field_path, stringify_field!(::0));

        let field = field!(<NestedName>::name::0);
        assert_eq!(field.field_path, stringify_field!(::name::0));
    }
}
