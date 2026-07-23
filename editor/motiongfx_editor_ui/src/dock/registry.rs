//! Registry of the window kinds that can be docked: what to call them
//! and how to build their content when a tab is materialized.

use std::sync::Arc;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::reactive::BevyUi;

/// Builds a window's content as kernel nodes, so a panel can declare
/// its own `ui.watch` / `ui.bind` instead of reaching for the kernel
/// from outside.
///
/// `Arc`, not `Box`: the builder has to be cloned out of the registry
/// before it can run, because `Ui` holds the world mutably and the
/// registry borrow cannot survive the first spawn.
pub type DockWindowBuildFn =
    Arc<dyn for<'a> Fn(&mut BevyUi<'a>) + Send + Sync + 'static>;

pub struct DockWindowDescriptor {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub build: DockWindowBuildFn,
}

#[derive(Resource, Default)]
pub struct WindowRegistry {
    windows: Vec<DockWindowDescriptor>,
    index: HashMap<String, usize>,
}

impl WindowRegistry {
    pub fn register(&mut self, descriptor: DockWindowDescriptor) {
        // Replace in place on a duplicate id, or both descriptors
        // linger in `windows` while `index` points only at the newer.
        if let Some(&idx) = self.index.get(&descriptor.id) {
            self.windows[idx] = descriptor;
            return;
        }
        let idx = self.windows.len();
        self.index.insert(descriptor.id.clone(), idx);
        self.windows.push(descriptor);
    }

    /// Remove a window by id. Returns true if the window was found.
    /// Rebuilds the id -> index mapping after removal.
    pub fn unregister(&mut self, id: &str) -> bool {
        let Some(idx) = self.index.remove(id) else {
            return false;
        };
        self.windows.remove(idx);
        // Re-index remaining entries since positions shifted.
        self.index.clear();
        for (i, w) in self.windows.iter().enumerate() {
            self.index.insert(w.id.clone(), i);
        }
        true
    }

    pub fn get(&self, id: &str) -> Option<&DockWindowDescriptor> {
        self.index.get(id).map(|&i| &self.windows[i])
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = &DockWindowDescriptor> {
        self.windows.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(id: &str, name: &str) -> DockWindowDescriptor {
        DockWindowDescriptor {
            id: id.to_string(),
            name: name.to_string(),
            icon: None,
            build: Arc::new(|_| {}),
        }
    }

    #[test]
    fn register_duplicate_id_replaces_in_place() {
        let mut reg = WindowRegistry::default();
        reg.register(descriptor("x", "first"));
        reg.register(descriptor("x", "second"));

        // The duplicate must not leave a stale descriptor behind.
        assert_eq!(reg.iter().count(), 1);
        assert_eq!(
            reg.get("x").map(|d| d.name.as_str()),
            Some("second")
        );
    }

    #[test]
    fn unregister_after_duplicate_leaves_nothing() {
        let mut reg = WindowRegistry::default();
        reg.register(descriptor("x", "first"));
        reg.register(descriptor("x", "second"));

        assert!(reg.unregister("x"));
        assert_eq!(reg.iter().count(), 0);
        assert!(reg.get("x").is_none());
    }
}
