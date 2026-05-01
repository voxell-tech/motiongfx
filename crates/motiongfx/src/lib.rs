#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

pub mod action;
pub mod ease;
pub mod pipeline;
pub mod registry;
pub mod sequence;
pub mod subject;
pub mod timeline;
pub mod track;

// Re-exports field_path as it is essential for motiongfx to work!
pub use field_path;

pub mod prelude {
    pub use field_path::field_accessor::FieldAccessor;

    pub use crate::ThreadSafe;
    pub use crate::action::{
        Action, ActionBuilder, ActionId, EaseFn, InterpActionBuilder,
        InterpFn,
    };
    pub use crate::ease;
    pub use crate::path;
    pub use crate::pipeline::{PipelineKey, SubjectSource};
    pub use crate::registry::{
        AccessorRegistry, PipelineRegistry, Registry,
    };
    pub use crate::timeline::{Timeline, TimelineBuilder};
    pub use crate::track::{Track, TrackFragment, TrackOrdering};
}

/// See [`field_path::field_accessor!`].
///
/// This macro just forwards the tokens to the mentioned macro.
///
/// ## Example
///
/// ```
/// use motiongfx::path;
///
/// struct Foo(u32);
///
/// let path = path!(<Foo>::0);
/// ```
#[macro_export]
macro_rules! path {
    ($($t:tt)*) => {
        $crate::field_path::field_accessor!($($t)*)
    };
}

/// Auto trait for types that implements [`Send`] + [`Sync`] +
/// `'static`.
pub trait ThreadSafe: Send + Sync + 'static {}

impl<T> ThreadSafe for T where T: Send + Sync + 'static {}

#[cfg(test)]
mod tests {
    use crate::path;

    // ── path! macro ───────────────────────────────────────────────────────────

    struct Foo {
        pub x: f32,
        pub y: f32,
    }

    struct Nested {
        pub inner: Foo,
    }

    /// Verify the macro compiles and the resulting accessor reads the
    /// correct field.
    #[test]
    fn path_macro_accesses_top_level_field() {
        let field_acc = path!(<Foo>::x);
        let subject = Foo { x: 3.14, y: 0.0 };
        assert_eq!(*field_acc.accessor.get_ref(&subject), 3.14);
    }

    #[test]
    fn path_macro_accesses_different_field_in_same_struct() {
        let field_acc_x = path!(<Foo>::x);
        let field_acc_y = path!(<Foo>::y);

        let subject = Foo { x: 1.0, y: 2.0 };
        assert_eq!(*field_acc_x.accessor.get_ref(&subject), 1.0);
        assert_eq!(*field_acc_y.accessor.get_ref(&subject), 2.0);
    }

    #[test]
    fn path_macro_accesses_nested_field() {
        let field_acc = path!(<Nested>::inner::x);
        let subject = Nested {
            inner: Foo { x: 42.0, y: 0.0 },
        };
        assert_eq!(*field_acc.accessor.get_ref(&subject), 42.0);
    }

    #[test]
    fn path_macro_mutates_field_via_accessor() {
        let field_acc = path!(<Foo>::x);
        let mut subject = Foo { x: 0.0, y: 0.0 };
        *field_acc.accessor.get_mut(&mut subject) = 99.0;
        assert_eq!(subject.x, 99.0);
    }

    #[test]
    fn path_macro_produces_distinct_fields_for_different_paths() {
        let field_x = path!(<Foo>::x);
        let field_y = path!(<Foo>::y);
        // Different fields must have different untyped field keys.
        assert_ne!(
            field_x.field.untyped(),
            field_y.field.untyped(),
            "x and y fields should have distinct untyped keys"
        );
    }

    #[test]
    fn path_macro_same_path_produces_equal_fields() {
        let a = path!(<Foo>::x);
        let b = path!(<Foo>::x);
        assert_eq!(
            a.field.untyped(),
            b.field.untyped(),
            "Same path should produce equal untyped field"
        );
    }
}
