use alloc::boxed::Box;
use core::any::{Any, TypeId};

use hashbrown::HashMap;

/// A type-keyed singleton map, playing the role of a `bevy_ecs`
/// `Resource` store without depending on `bevy_ecs`.
#[derive(Default)]
pub(crate) struct Resources {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Resources {
    pub(crate) fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.map.get(&TypeId::of::<T>())?.downcast_ref()
    }

    pub(crate) fn get_mut<T: Any + Send + Sync>(
        &mut self,
    ) -> Option<&mut T> {
        self.map.get_mut(&TypeId::of::<T>())?.downcast_mut()
    }

    pub(crate) fn get_or_insert_with<T: Any + Send + Sync>(
        &mut self,
        f: impl FnOnce() -> T,
    ) -> &mut T {
        self.map
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(f()))
            .downcast_mut()
            .expect(
                "TypeId key always matches the boxed value's type",
            )
    }

    pub(crate) fn remove<T: Any + Send + Sync>(
        &mut self,
    ) -> Option<T> {
        let boxed = self.map.remove(&TypeId::of::<T>())?;
        boxed.downcast::<T>().ok().map(|value| *value)
    }
}
