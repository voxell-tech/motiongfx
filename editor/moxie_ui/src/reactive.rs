//! The signals a build or a binding fires on.
//!
//! The kernel itself is [`bevy_fynix`]; what lives here is the
//! predicates the editor asks it to watch, none of which know
//! anything about the kernel: each is a
//! `FnMut(WorldNodeRef<FynixHost>) -> bool` that answers "has this changed
//! since I last looked".

use bevy::ecs::change_detection::{ComponentTicks, Tick};
use bevy::prelude::*;
use fynix::WorldNodeRef;

// `watch_root` stays generic over `Theme`, same as `bevy_fynix`
// itself: `build`'s own type already fixes `Theme` to `EditorTheme`
// for the compiler to infer.
pub use bevy_fynix::{FynixSet, watch_root};

use crate::theme::EditorTheme;

/// Fixes [`bevy_fynix::host::BevyHost`]'s theme to [`EditorTheme`].
pub type FynixHost = bevy_fynix::host::BevyHost<EditorTheme>;
/// [`fynix::ui::Build`] against [`FynixHost`], for an element's own
/// `fn build`.
pub type FynixBuild<'a, E> = fynix::ui::Build<'a, FynixHost, E>;
pub type BevyUi<'a> = bevy_fynix::BevyUi<'a, EditorTheme>;
pub type FynixPlugin = bevy_fynix::FynixPlugin<EditorTheme>;

/// Fires when `R` changed since the last poll. Also fires on the first
/// poll, so a binding starts out in sync with the world.
pub fn resource_changed<R: Resource>()
-> impl for<'w> FnMut(WorldNodeRef<'w, FynixHost>) -> bool
+ Send
+ Sync
+ 'static {
    let mut seen: Option<Tick> = None;
    move |WorldNodeRef { world, .. }| {
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
/// Watching a whole resource rebuilds on any field change: dragging a
/// splitter nudges `DockTree`'s fractions every frame. Projecting to
/// the structural part skips that; a `bind` carries the ratio
/// instead.
pub fn structure_changed<R: Resource, K>(
    project: impl Fn(&R) -> K + Send + Sync + 'static,
) -> impl for<'w> FnMut(WorldNodeRef<'w, FynixHost>) -> bool
+ Send
+ Sync
+ 'static
where
    K: PartialEq + Send + Sync + 'static,
{
    let mut seen: Option<K> = None;
    move |WorldNodeRef { world, .. }| {
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
/// For entity state, where there is no tick to compare. `read` runs
/// every flush, so keep it cheap: resolve entities outside the
/// closure, since a predicate only has `&World` to scan with.
pub fn value_changed<T>(
    read: impl Fn(&World, Entity) -> T + Send + Sync + 'static,
) -> impl for<'w> FnMut(WorldNodeRef<'w, FynixHost>) -> bool
+ Send
+ Sync
+ 'static
where
    T: PartialEq + Send + Sync + 'static,
{
    let mut seen: Option<T> = None;
    move |WorldNodeRef { world, node }| {
        let current = read(world, node);
        let fired = seen.as_ref() != Some(&current);
        seen = Some(current);
        fired
    }
}

/// Fires when the node's `C` was written since the last poll, and on
/// the first poll.
///
/// Rides the tick. Reach for [`value_changed`] when the value itself
/// is what matters.
pub fn component_changed<C: Component>()
-> impl for<'w> FnMut(WorldNodeRef<'w, FynixHost>) -> bool
+ Send
+ Sync
+ 'static {
    let mut seen: Option<Option<Tick>> = None;
    move |WorldNodeRef { world, node }| {
        let current = component_tick::<C>(world, node);
        let fired = seen != Some(current);
        seen = Some(current);
        fired
    }
}

/// Same, for `C` on some other entity than the node.
pub fn component_changed_on<C: Component>(
    entity: Entity,
) -> impl for<'w> FnMut(WorldNodeRef<'w, FynixHost>) -> bool
+ Send
+ Sync
+ 'static {
    let mut changed =
        tick_changed(move |world| component_tick::<C>(world, entity));
    move |WorldNodeRef { world, .. }| changed(world)
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

/// Fires when `read`'s tick differs from the last poll, and on the
/// first poll.
pub(crate) fn tick_changed(
    mut read: impl FnMut(&World) -> Option<Tick> + Send + Sync + 'static,
) -> impl FnMut(&World) -> bool + Send + Sync + 'static {
    let mut seen: Option<Option<Tick>> = None;
    move |world| {
        let current = read(world);
        let fired = seen != Some(current);
        seen = Some(current);
        fired
    }
}

/// Fires when the current `S` differs from the last poll.
pub fn state_changed<S: States>()
-> impl for<'w> FnMut(WorldNodeRef<'w, FynixHost>) -> bool
+ Send
+ Sync
+ 'static {
    let mut seen: Option<S> = None;
    move |WorldNodeRef { world, .. }| {
        let current =
            world.get_resource::<State<S>>().map(|s| s.get().clone());
        let fired = seen != current;
        seen = current;
        fired
    }
}
