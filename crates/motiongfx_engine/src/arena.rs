//! TODO: Create an arena that could store different types that can
//! be mapped by an id (`ArenaId`).
//!
//! There should be a method to add, remove, and get the value...
//! ```ignore
//! fn add<T>(value: T) -> ArenaId { .. }
//! fn remove<T>(id: &ArenaId) -> bool { .. }
//! fn get<T>(id: &ArenaId) -> Option<&T> { .. }
//! ```
//!
//! Use `bevy::ptr::OwningPtr` to store the drop function for the values.
//!
//! Example on how it can be used:
//! ```ignore
//! /// # Safety
//! ///
//! /// `x` must point to a valid value of type `T`.
//! unsafe fn drop_ptr<T>(x: OwningPtr<'_>) {
//!     // SAFETY: Contract is required to be upheld by the caller.
//!     unsafe {
//!         x.drop_as::<T>();
//!     }
//! }
//!
//! pub fn new<T>() -> Self {
//!     Self {
//!         // ..
//!         type_id: Some(TypeId::of::<T>()),
//!         layout: Layout::new::<T>(),
//!         // Drop pointer is stored here!
//!         drop: needs_drop::<T>().then_some(Self::drop_ptr::<T> as _),
//!         // ..
//!     }
//! }
//! ```
//!
//! Draft:
//! ```ignore
//! pub struct ArenaId {
//!     type_id: TypeId,
//!     uid: u64,
//! }
//!
//! pub struct ArenaSpan {
//!     offset: usize,
//!     len: usize,
//! }
//!
//! pub struct Arena {
//!     storage: NonNull<u8>,
//!     spans: HashMap<ArenaId, ArenaSpan>,
//!     type_infos: HashMap<TypeId, TypeInfo>,
//! }
//!
//! pub struct TypeInfo {
//!     drop: Option<unsafe fn(OwningPtr<'_>)>,
//!     layout: Layout,
//! }
//!
//! struct DenseArenaSpan {
//!     storage_span: usize,
//!     info_span: usize,
//! }
//!
//! /// A dense arena which is also immutable.
//! pub struct DenseArena {
//!     storage: Box<[u8]>,
//!     spans: Box<[ArenaSpan]>,
//!     type_infos: Box<[TypeInfo]>,
//!     dense_map: HashMap<ArenaId, DenseArenaSpan>,
//! }
//! ```
//!
//! Potential usage:
//! - Replace the world in `FieldRegistry`.
//! - `ActionSpan` can reference to an arena in `Timeline`/`Track`,
//!   allowing baking/sampling to happen locally.

use core::alloc::Layout;
use core::any::TypeId;
use core::ptr;
use core::ptr::NonNull;

use bevy::platform::collections::HashMap;
use bevy::ptr::{OwningPtr, Ptr, PtrMut};

/// Unique identifier for values stored in the arena.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ArenaId {
    type_id: TypeId,
    uid: u64,
}

/// Span of memory in the arena buffer.
#[derive(Clone, Copy, Debug)]
pub struct ArenaSpan {
    offset: usize,
    len: usize,
}

/// Metadata for a stored type.
pub struct TypeInfo {
    pub drop: Option<unsafe fn(OwningPtr<'_>)>,
    pub layout: Layout,
}

impl TypeInfo {
    /// # Safety
    ///
    /// `x` must point to a valid value of type `T`.
    unsafe fn drop_ptr<T>(x: OwningPtr<'_>) {
        // SAFETY: Contract is upheld by caller.
        unsafe { x.drop_as::<T>() };
    }

    pub fn new<T: 'static>() -> Self {
        Self {
            layout: Layout::new::<T>(),
            drop: core::mem::needs_drop::<T>()
                .then_some(Self::drop_ptr::<T> as _),
        }
    }
}

/// Heterogenous arena that can store any `T: 'static`.
#[derive(Default)]
pub struct Arena {
    storage: Vec<u8>,
    spans: HashMap<ArenaId, ArenaSpan>,
    type_infos: HashMap<TypeId, TypeInfo>,
    next_uid: u64,
}

impl Arena {
    pub fn new() -> Self {
        Self {
            storage: Vec::new(),
            spans: HashMap::new(),
            type_infos: HashMap::new(),
            next_uid: 0,
        }
    }

    pub fn add<T: 'static>(&mut self, value: T) -> ArenaId {
        let type_id = TypeId::of::<T>();
        let info = self
            .type_infos
            .entry(type_id)
            .or_insert_with(TypeInfo::new::<T>);

        let offset = self.storage.len();
        let size = info.layout.size();

        // expand storage
        let ptr = self.storage.as_mut_ptr();
        let len = self.storage.len();
        let cap = self.storage.capacity();
        let new_len = len + size;
        if new_len > cap {
            self.storage.reserve(size);
        }

        unsafe {
            let dst =
                self.storage.as_mut_ptr().add(offset).cast::<T>();
            ptr::write(dst, value);
            self.storage.set_len(new_len);
        }

        let id = ArenaId {
            type_id,
            uid: self.next_uid,
        };
        self.next_uid += 1;

        self.spans.insert(id, ArenaSpan { offset, len: size });
        id
    }

    pub fn get<T: 'static>(&self, id: &ArenaId) -> Option<&T> {
        if id.type_id != TypeId::of::<T>() {
            return None;
        }
        let span = self.spans.get(id)?;

        unsafe {
            let ptr =
                self.storage.as_ptr().add(span.offset).cast::<T>();
            Some(&*ptr)
        }
    }

    pub fn remove<T: 'static>(&mut self, id: &ArenaId) -> bool {
        let span = match self.spans.remove(id) {
            Some(s) => s,
            None => return false,
        };
        if id.type_id != TypeId::of::<T>() {
            return false;
        }
        let info = self.type_infos.get(&id.type_id).unwrap();

        unsafe {
            let ptr = NonNull::new_unchecked(
                self.storage.as_mut_ptr().add(span.offset),
            );
            let owning = OwningPtr::new(ptr);
            if let Some(drop_fn) = info.drop {
                drop_fn(owning);
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_get_u32() {
        let mut arena = Arena::new();
        let id = arena.add(42u32);

        let value = arena.get::<u32>(&id).unwrap();
        assert_eq!(*value, 42);
    }

    #[test]
    fn add_and_get_multiple_types() {
        let mut arena = Arena::new();
        let id1 = arena.add(123u32);
        let id2 = arena.add(String::from("hello"));

        assert_eq!(arena.get::<u32>(&id1), Some(&123));
        assert_eq!(
            arena.get::<String>(&id2),
            Some(&String::from("hello"))
        );
    }

    #[test]
    fn wrong_type_returns_none() {
        let mut arena = Arena::new();
        let id = arena.add(99u32);

        // Try to get it as the wrong type
        assert!(arena.get::<String>(&id).is_none());
    }

    #[test]
    fn remove_returns_true_when_successful() {
        let mut arena = Arena::new();
        let id = arena.add(7u32);

        assert!(arena.remove::<u32>(&id));
        assert!(arena.get::<u32>(&id).is_none());
    }

    #[test]
    fn remove_wrong_type_fails() {
        let mut arena = Arena::new();
        let id = arena.add(7u32);

        // Wrong type removal should fail
        assert!(!arena.remove::<String>(&id));
    }

    #[test]
    fn drop_is_called_for_types_with_destructor() {
        use std::cell::RefCell;
        use std::rc::Rc;

        struct Tracker(Rc<RefCell<u32>>);

        impl Drop for Tracker {
            fn drop(&mut self) {
                *self.0.borrow_mut() += 1;
            }
        }

        let counter = Rc::new(RefCell::new(0));
        {
            let mut arena = Arena::new();
            let id = arena.add(Tracker(counter.clone()));

            assert_eq!(*counter.borrow(), 0);
            assert!(arena.remove::<Tracker>(&id));
        }
        // Tracker should have been dropped once
        assert_eq!(*counter.borrow(), 1);
    }

    #[test]
    fn ids_are_unique() {
        let mut arena = Arena::new();
        let id1 = arena.add(1u32);
        let id2 = arena.add(2u32);

        assert_ne!(id1.uid, id2.uid);
    }
}
