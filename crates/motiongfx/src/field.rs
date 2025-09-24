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

use bevy_ecs::prelude::*;

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
/// use motiongfx::field::{Field, field};
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
/// assert_eq!(name.field_path(), "::name");
/// assert_eq!(age.field_path(), "::age");
/// ```
#[derive(Component, Debug, Hash, PartialEq, Eq)]
pub struct Field<S, T> {
    /// The path of the target field in the source.
    ///
    /// Example: `Transform::translation::x` will have a field path
    /// of `"::translation::x"`.
    field_path: &'static str,
    _marker: PhantomData<(S, T)>,
}

impl<S, T> Field<S, T> {
    /// A field with a placeholder field path. This does not correspond
    /// to a vaild field path!
    pub const PLACEHOLDER: Self = Self::new("$");

    /// Construct a new [`Field`] from a raw field path string.
    ///
    /// Prefer the [`field!`] macro for type safety!
    pub const fn new(field_path: &'static str) -> Self {
        Self {
            field_path,
            _marker: PhantomData,
        }
    }

    /// Returns the raw field path string.
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

impl<S, T> Clone for Field<S, T> {
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
    _field_marker: fn(source: S) -> T,
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
            _field_marker: field_marker,
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
/// use motiongfx::field::{Field, field};
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
/// path, create one via [`Field::untyped`].
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
    pub fn new<S: 'static, T: 'static>(
        field_path: &'static str,
    ) -> Self {
        Self {
            source_id: TypeId::of::<S>(),
            target_id: TypeId::of::<T>(),
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

    /// Converts into a typed [`Field<S, T>`].
    ///
    /// # Panics
    ///
    /// Panics if the type does not match.
    pub fn typed<S: 'static, T: 'static>(self) -> Field<S, T> {
        assert_eq!(TypeId::of::<S>(), self.source_id);
        assert_eq!(TypeId::of::<T>(), self.target_id);
        self.typed_unchecked()
    }

    /// Converts into a typed [`Field<S, T>`] without type checks.
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

/// Stringify a field path into its canonical string form.
///
/// This macro is used within the [`field!`] macro for supporting
/// auto-completion of nested fields while still being able to generate
/// "stringify" field paths from raw tokens!
///
/// # Example
///
/// ```
/// use motiongfx::field::stringify_field;
///
/// let stringify = stringify_field!(::translation::x);
/// assert_eq!(stringify, "::translation::x");
/// ```
#[macro_export]
macro_rules! stringify_field {
    ($(::$field:tt)*) => {
        concat!($("::", stringify!($field),)*)
    };
}
pub use stringify_field;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(PartialEq, Debug, Clone)]
    struct Foo(u32);

    #[derive(PartialEq, Debug, Clone)]
    struct NestedFoo {
        inner: Foo,
    }

    #[test]
    fn test_field() {
        let field = field!(<Foo>);
        assert_eq!(field.field_path, "");

        let field = field!(<Foo>::0);
        assert_eq!(field.field_path, stringify_field!(::0));

        let field = field!(<NestedFoo>::inner::0);
        assert_eq!(field.field_path, stringify_field!(::inner::0));
    }
}
