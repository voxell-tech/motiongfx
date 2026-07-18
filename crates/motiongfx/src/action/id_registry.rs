use core::any::TypeId;

use hashbrown::HashMap;

use crate::resources::Resources;
use crate::subject::SubjectId;

/// A type-erased unique Id in the [`IdRegistry`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct UId(pub(super) u64);

/// A type-erased [`UId`] map and generator for each unique
/// [`SubjectId`]s. It also performs book keeping for all id instances
/// and remove them when there is none left.
pub struct IdRegistry<I: SubjectId> {
    /// Maps `SubjectId`s to [`UId`]s .
    uid_map: HashMap<I, UId>,
    /// Maps [`UId`]s to `SubjectId`s.
    id_map: HashMap<UId, I>,
    /// The number of instances using the same [`UId`].
    instance_counts: HashMap<UId, u32>,
    /// The next [`UId`], incremented on every new [`UId`] created.
    next_uid: UId,
}

impl<I: SubjectId> IdRegistry<I> {
    pub fn new() -> Self {
        Self {
            uid_map: HashMap::new(),
            id_map: HashMap::new(),
            instance_counts: HashMap::new(),
            next_uid: UId(0),
        }
    }

    /// Registers the [`SubjectId`] with an intial instance count of 1
    /// if it doesn't exist yet, otherwise, increase the existing
    /// instance count.
    ///
    /// Returns the [`UId`] of the associated [`SubjectId`].
    pub fn register_instance(&mut self, id: I) -> UId {
        let uid = *self.uid_map.entry(id).or_insert_with(|| {
            self.next_uid.0 += 1;
            self.id_map.insert(self.next_uid, id);
            // Starts at 0: the unconditional `+= 1` below always
            // runs next, bringing a freshly registered id to the
            // documented initial count of 1.
            self.instance_counts.insert(self.next_uid, 0);
            self.next_uid
        });

        // SAFETY: `uid_counts` is added for every new UId!
        *self.instance_counts.get_mut(&uid).unwrap() += 1;

        uid
    }

    /// Reduce the instance count of a [`SubjectId`] associated with
    /// the provided [`UId`]. When the instance count reaches 0, the
    /// entire registry will be erased.
    ///
    /// Returns `true` if the instance is being successfully removed,
    /// `false` if the registry doesn't exist in the first place.
    pub fn remove_instance(&mut self, uid: &UId) -> bool {
        let Some(count) = self.instance_counts.get_mut(uid) else {
            return false;
        };

        *count -= 1;

        // Remove the underlying data when it's the last instance.
        if *count == 0 {
            let id = self.id_map.get(uid).unwrap();
            self.uid_map.remove(id);
            self.id_map.remove(uid);
            self.instance_counts.remove(uid);
        }

        true
    }

    /// Checks if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.uid_map.is_empty()
    }

    pub fn get_uid(&self, id: &I) -> Option<&UId> {
        self.uid_map.get(id)
    }

    pub fn get_id(&self, uid: &UId) -> Option<&I> {
        self.id_map.get(uid)
    }
}

impl<I: SubjectId> Default for IdRegistry<I> {
    fn default() -> Self {
        Self::new()
    }
}

/// A type-erased cleanup function, run when an action is removed to
/// decrement the spawning [`SubjectId`]'s [`IdRegistry`] entry.
///
/// `I` is baked in via monomorphization at insertion time (see
/// [`cleanup_fn`]); the caller only needs to supply the removed
/// action's [`UId`] when calling it back.
pub(crate) type Cleanup = fn(&mut Resources, UId);

/// One [`Cleanup`] per distinct [`SubjectId`] type `I` used across
/// the whole [`ActionTable`](crate::action::ActionTable), keyed by
/// `I`'s [`TypeId`]. Stored as a [`Resources`] entry itself, right
/// alongside each `I`'s [`IdRegistry`], rather than duplicated once
/// per action — the function is identical for every action sharing
/// the same `I`.
pub(crate) type CleanupRegistry = HashMap<TypeId, Cleanup>;

pub(crate) fn cleanup_fn<I: SubjectId>(
    resources: &mut Resources,
    uid: UId,
) {
    let Some(registry) = resources.get_mut::<IdRegistry<I>>() else {
        return;
    };
    registry.remove_instance(&uid);

    if registry.is_empty() {
        resources.remove::<IdRegistry<I>>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_registry_is_empty() {
        let registry = IdRegistry::<u32>::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn register_instance_resolves_both_ways() {
        let mut registry = IdRegistry::new();
        let uid = registry.register_instance(7u32);

        assert_eq!(registry.get_uid(&7u32), Some(&uid));
        assert_eq!(registry.get_id(&uid), Some(&7u32));
        assert!(!registry.is_empty());
    }

    #[test]
    fn register_instance_same_subject_returns_same_uid() {
        let mut registry = IdRegistry::new();
        let uid1 = registry.register_instance(7u32);
        let uid2 = registry.register_instance(7u32);

        assert_eq!(uid1, uid2);
    }

    #[test]
    fn register_instance_distinct_subjects_get_distinct_uids() {
        let mut registry = IdRegistry::new();
        let uid1 = registry.register_instance(1u32);
        let uid2 = registry.register_instance(2u32);

        assert_ne!(uid1, uid2);
    }

    #[test]
    fn remove_instance_erases_entry_after_single_registration() {
        let mut registry = IdRegistry::new();
        let uid = registry.register_instance(7u32);

        assert!(registry.remove_instance(&uid));
        assert!(registry.is_empty());
        assert_eq!(registry.get_id(&uid), None);
        assert_eq!(registry.get_uid(&7u32), None);
    }

    #[test]
    fn remove_instance_ref_counts_multiple_registrations() {
        let mut registry = IdRegistry::new();
        let uid1 = registry.register_instance(7u32);
        let uid2 = registry.register_instance(7u32);
        assert_eq!(uid1, uid2);

        // One instance removed, one remains: entry must survive.
        assert!(registry.remove_instance(&uid1));
        assert!(!registry.is_empty());
        assert_eq!(registry.get_id(&uid1), Some(&7u32));

        // Last instance removed: entry is erased.
        assert!(registry.remove_instance(&uid1));
        assert!(registry.is_empty());
        assert_eq!(registry.get_id(&uid1), None);
    }

    #[test]
    fn remove_instance_unknown_uid_returns_false() {
        let mut registry = IdRegistry::<u32>::new();
        let uid = registry.register_instance(1u32);
        registry.remove_instance(&uid);

        // Already removed: a second removal finds nothing.
        assert!(!registry.remove_instance(&uid));
    }

    #[test]
    fn registrations_of_different_subjects_are_independent() {
        let mut registry = IdRegistry::new();
        let uid1 = registry.register_instance(1u32);
        let uid2 = registry.register_instance(2u32);

        assert!(registry.remove_instance(&uid1));
        assert!(!registry.is_empty());
        assert_eq!(registry.get_id(&uid1), None);
        assert_eq!(registry.get_id(&uid2), Some(&2u32));
    }
}
