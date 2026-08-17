//! The signals a build or a binding fires on.
//!
//! The kernel itself is [`bevy_fynix`]; what lives here is the
//! predicates the editor asks it to watch, none of which know
//! anything about the kernel: each is a `FnMut(&World, Entity)
//! -> bool` that answers "has this changed since I last looked".

use bevy::ecs::change_detection::{ComponentTicks, Tick};
use bevy::prelude::*;

pub use bevy_fynix::host::BevyHost;
pub use bevy_fynix::{BevyUi, FynixPlugin, FynixSet, watch_root};

/// Fires when `R` changed since the last poll. Also fires on the first
/// poll, so a binding starts out in sync with the world.
pub fn resource_changed<R: Resource>()
-> impl FnMut(&World, Entity) -> bool + Send + Sync + 'static {
    let mut seen: Option<Tick> = None;
    move |world, _| {
        let Some(ticks) = world.get_resource_change_ticks::<R>()
        else {
            return false;
        };
        let ComponentTicks { changed, .. } = ticks;
        let fired = seen != Some(changed);
        seen = Some(changed);
        fired
    }
}

/// Fires when a *projection* of `R` changes, ignoring every other
/// mutation.
///
/// This is what lets `watch` mean "structure" and `bind` mean "value".
/// Watching a whole resource rebuilds on any field change: dragging a
/// splitter nudges `DockTree`'s fractions every frame, which would
/// rebuild the layout every frame. Project to the structural part and
/// the drag fires nothing; a `bind` carries the ratio instead.
pub fn structure_changed<R: Resource, K>(
    project: impl Fn(&R) -> K + Send + Sync + 'static,
) -> impl FnMut(&World, Entity) -> bool + Send + Sync + 'static
where
    K: PartialEq + Send + Sync + 'static,
{
    let mut seen: Option<K> = None;
    move |world, _| {
        let Some(resource) = world.get_resource::<R>() else {
            return false;
        };
        let current = project(resource);
        let fired = seen.as_ref() != Some(&current);
        seen = Some(current);
        fired
    }
}

/// Fires when `read`'s value differs from the last poll.
///
/// For signals that are entity state rather than a resource, where
/// there is no tick to compare. `read` runs every flush, so it must be
/// cheap: resolve entities *outside* the closure (a registering system
/// has queries; a predicate only has `&World`, where finding an entity
/// by component means scanning the whole world).
pub fn value_changed<T>(
    read: impl Fn(&World, Entity) -> T + Send + Sync + 'static,
) -> impl FnMut(&World, Entity) -> bool + Send + Sync + 'static
where
    T: PartialEq + Send + Sync + 'static,
{
    let mut seen: Option<T> = None;
    move |world, node| {
        let current = read(world, node);
        let fired = seen.as_ref() != Some(&current);
        seen = Some(current);
        fired
    }
}

/// Fires when the node's `C` was written since the last poll, and on
/// the first poll.
///
/// State belonging to one widget instance lives on its own node
/// rather than in a `Resource` every instance would share. Rides the
/// tick, so [`value_changed`] is what to reach for when the value is
/// what matters.
pub fn component_changed<C: Component>()
-> impl FnMut(&World, Entity) -> bool + Send + Sync + 'static {
    let mut seen: Option<Option<Tick>> = None;
    move |world, node| {
        let current = component_tick::<C>(world, node);
        let fired = seen != Some(current);
        seen = Some(current);
        fired
    }
}

/// Same, for `C` on some other entity than the node.
pub fn component_changed_on<C: Component>(
    entity: Entity,
) -> impl FnMut(&World, Entity) -> bool + Send + Sync + 'static {
    let mut seen: Option<Option<Tick>> = None;
    move |world, _| {
        let current = component_tick::<C>(world, entity);
        let fired = seen != Some(current);
        seen = Some(current);
        fired
    }
}

/// When `C` on `entity` was last written.
fn component_tick<C: Component>(
    world: &World,
    entity: Entity,
) -> Option<Tick> {
    let ComponentTicks { changed, .. } =
        world.get_entity(entity).ok()?.get_change_ticks::<C>()?;
    Some(changed)
}

/// Fires when the current `S` differs from the last poll.
pub fn state_changed<S: States>()
-> impl FnMut(&World, Entity) -> bool + Send + Sync + 'static {
    let mut seen: Option<S> = None;
    move |world, _| {
        let current =
            world.get_resource::<State<S>>().map(|s| s.get().clone());
        let fired = seen != current;
        seen = current;
        fired
    }
}
