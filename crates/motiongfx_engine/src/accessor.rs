//! Accessor system for mapping source structures to target fields.
//!
//! This module provides both typed [`Accessor`]s (compile-time
//! source/target types) and type-erased [`UntypedAccessor`]s
//! (runtime checked).
//!
//! They can be registered and retrieved via the [`AccessorRegistry`].

use core::any::TypeId;
use core::hash::Hash;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// A typed accessor to a field of type `T` within a source type `S`.
///
/// This holds both immutable and mutable function pointers, which
/// allows retrieving references to the target field inside a source.
///
/// # Example
/// ```
/// use motiongfx_engine::accessor::Accessor;
///
/// #[derive(Default)]
/// struct Foo { value: i32 }
///
/// fn ref_fn(s: &Foo) -> &i32 { &s.value }
/// fn mut_fn(s: &mut Foo) -> &mut i32 { &mut s.value }
///
/// let accessor = Accessor { ref_fn, mut_fn };
/// let mut foo = Foo { value: 42 };
///
/// assert_eq!(*(accessor.ref_fn)(&foo), 42);
/// *(accessor.mut_fn)(&mut foo) = 999;
/// assert_eq!(foo.value, 999);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Accessor<S: 'static, T: 'static> {
    pub ref_fn: fn(&S) -> &T,
    pub mut_fn: fn(&mut S) -> &mut T,
}

impl<S, T> Accessor<S, T> {
    pub fn untyped(self) -> UntypedAccessor {
        UntypedAccessor::new(self.ref_fn, self.mut_fn)
    }
}

/// A type-erased version of [`Accessor`].
///
/// Stores the raw function pointers as `*const ()` along with
/// [`TypeId`]s of both source and target. This allows
/// dynamically checking and restoring the original [`Accessor`].
#[derive(Debug, Clone, Copy)]
pub struct UntypedAccessor {
    ref_fn: *const (),
    mut_fn: *const (),
    source_id: TypeId,
    target_id: TypeId,
}

impl UntypedAccessor {
    /// Create a new type-erased accessor from a typed accessor pair.
    pub fn new<S: 'static, T: 'static>(
        ref_fn: fn(&S) -> &T,
        mut_fn: fn(&mut S) -> &mut T,
    ) -> Self {
        Self {
            ref_fn: ref_fn as *const (),
            mut_fn: mut_fn as *const (),
            source_id: TypeId::of::<S>(),
            target_id: TypeId::of::<T>(),
        }
    }

    /// Re-interpret this accessor as a typed [`Accessor`] without
    /// checking [`TypeId`]s. Caller must guarantee type correctness.
    ///
    /// # Safety
    ///
    /// Undefined behavior if `S` and `T` do not match the types used
    /// when constructing this accessor.
    pub unsafe fn typed_unchecked<S, T>(self) -> Accessor<S, T> {
        Accessor {
            ref_fn: core::mem::transmute::<*const (), fn(&S) -> &T>(
                self.ref_fn,
            ),
            mut_fn: core::mem::transmute::<
                *const (),
                fn(&mut S) -> &mut T,
            >(self.mut_fn),
        }
    }

    /// Attempt to re-interpret this accessor as a typed [`Accessor`],
    /// returning `None` if the [`TypeId`]s do not match.
    pub fn typed<S, T>(self) -> Option<Accessor<S, T>> {
        if self.source_id == TypeId::of::<S>()
            && self.target_id == TypeId::of::<T>()
        {
            unsafe {
                return Some(self.typed_unchecked());
            }
        }

        None
    }
}

impl<S, T> From<Accessor<S, T>> for UntypedAccessor {
    fn from(accessor: Accessor<S, T>) -> Self {
        accessor.untyped()
    }
}

/// A registry mapping keys to [`UntypedAccessor`]s.
///
/// Provides convenient insertion of typed accessors and
/// retrieval as typed [`Accessor`]s with runtime checking.
///
/// # Example
/// ```
/// use motiongfx_engine::accessor::{Accessor, AccessorRegistry};
///
/// #[derive(Default)]
/// struct Foo { value: i32 }
///
/// fn ref_fn(s: &Foo) -> &i32 { &s.value }
/// fn mut_fn(s: &mut Foo) -> &mut i32 { &mut s.value }
///
/// let mut registry = AccessorRegistry::new();
/// registry.insert("foo", Accessor { ref_fn, mut_fn });
///
/// let accessor = registry.get::<Foo, i32>(&"foo").unwrap();
/// let mut foo = Foo { value: 123 };
///
/// assert_eq!(*(accessor.ref_fn)(&foo), 123);
/// *(accessor.mut_fn)(&mut foo) = 999;
/// assert_eq!(foo.value, 999);
/// ```
#[derive(Resource, Debug)]
pub struct AccessorRegistry<K> {
    accessors: HashMap<K, UntypedAccessor>,
}

impl<K> AccessorRegistry<K> {
    /// Construct an empty [`AccessorRegistry`].
    pub fn new() -> Self {
        Self::default()
    }
}

impl<K: Eq + Hash> AccessorRegistry<K> {
    /// Insert an [`UntypedAccessor`] for a given key.
    pub fn insert(
        &mut self,
        key: K,
        accessor: impl Into<UntypedAccessor>,
    ) {
        self.accessors.insert(key, accessor.into());
    }

    /// Retrieve a typed [`Accessor`] from the registry.
    ///
    /// Returns an [`AccessorRegErr`] if the key does not exist or
    /// if the types do not match.
    pub fn get<S: 'static, T: 'static>(
        &self,
        key: &K,
    ) -> Result<Accessor<S, T>, AccessorRegErr> {
        self.accessors
            .get(key)
            .ok_or(AccessorRegErr::KeyNotFound)?
            .typed()
            .ok_or(AccessorRegErr::TypeMismatch)
    }
}

impl<K> Default for AccessorRegistry<K> {
    fn default() -> Self {
        Self {
            accessors: HashMap::new(),
        }
    }
}

/// Possible error variants when getting an [`Accessor`]
/// from the [`AccessorRegistry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessorRegErr {
    /// The requested key was not found in the registry.
    KeyNotFound,
    /// The [`Accessor`] exists but the source/target types did
    /// not match.
    TypeMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Foo {
        x: i32,
        y: f32,
    }

    fn foo_x_ref(foo: &Foo) -> &i32 {
        &foo.x
    }
    fn foo_x_mut(foo: &mut Foo) -> &mut i32 {
        &mut foo.x
    }

    fn foo_y_ref(foo: &Foo) -> &f32 {
        &foo.y
    }
    fn foo_y_mut(foo: &mut Foo) -> &mut f32 {
        &mut foo.y
    }

    #[test]
    fn accessor_roundtrip_typed_untyped() {
        let acc: Accessor<Foo, i32> = Accessor {
            ref_fn: foo_x_ref,
            mut_fn: foo_x_mut,
        };

        let untyped = acc.untyped();
        let typed_back: Accessor<Foo, i32> = untyped.typed().unwrap();

        let mut foo = Foo { x: 42, y: 1.5 };

        assert_eq!((typed_back.ref_fn)(&foo), &42);

        let x_mut = (typed_back.mut_fn)(&mut foo);
        *x_mut = 99;

        assert_eq!(foo.x, 99);
    }

    #[test]
    fn untyped_typed_mismatch_fails() {
        let acc: Accessor<Foo, i32> = Accessor {
            ref_fn: foo_x_ref,
            mut_fn: foo_x_mut,
        };

        let untyped = acc.untyped();

        // Mismatched type parameters should return None
        let wrong: Option<Accessor<Foo, f32>> = untyped.typed();
        assert!(wrong.is_none());
    }

    #[test]
    fn registry_insert_and_get_success() {
        let mut registry: AccessorRegistry<&'static str> =
            AccessorRegistry::new();

        registry.insert(
            "foo_x",
            Accessor {
                ref_fn: foo_x_ref,
                mut_fn: foo_x_mut,
            },
        );

        registry.insert(
            "foo_y",
            Accessor {
                ref_fn: foo_y_ref,
                mut_fn: foo_y_mut,
            },
        );

        let mut foo = Foo { x: 10, y: 1.5 };

        let x_accessor = registry.get::<Foo, i32>(&"foo_x").unwrap();
        assert_eq!((x_accessor.ref_fn)(&foo), &10);

        let y_accessor = registry.get::<Foo, f32>(&"foo_y").unwrap();
        assert_eq!((y_accessor.ref_fn)(&foo), &1.5);

        // Mutate via accessor
        *(x_accessor.mut_fn)(&mut foo) = 77;
        *(y_accessor.mut_fn)(&mut foo) = 2.5;

        assert_eq!(foo.x, 77);
        assert_eq!(foo.y, 2.5);
    }

    #[test]
    fn registry_key_not_found_error() {
        let registry: AccessorRegistry<&'static str> =
            AccessorRegistry::new();

        let res = registry.get::<Foo, i32>(&"missing");
        assert!(matches!(res, Err(AccessorRegErr::KeyNotFound)));
    }

    #[test]
    fn registry_type_mismatch_error() {
        let mut registry: AccessorRegistry<&'static str> =
            AccessorRegistry::new();

        registry.insert(
            "foo_x",
            Accessor {
                ref_fn: foo_x_ref,
                mut_fn: foo_x_mut,
            },
        );

        let res = registry.get::<Foo, f32>(&"foo_x");
        assert!(matches!(res, Err(AccessorRegErr::TypeMismatch)));
    }
}
