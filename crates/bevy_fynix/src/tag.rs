//! Pointer events as tags.
//!
//! fynix knows nothing about pointers: it stores whatever tags a node
//! is given and lets each field's `on(...)` lines answer to them. The
//! mapping from Bevy's events to those tags is this module's whole
//! job, and it is one call per event - tags stack, so releasing drops
//! [`Pressed`] and leaves [`Hovered`] standing.

use core::marker::PhantomData;

use bevy_ecs::prelude::*;
use bevy_picking::events::{
    Cancel, Out, Over, Pointer, Press, Release,
};
use fynix::anim::Tag;
use fynix::element::Element;
use fynix::ui::{Build, ElementMut};

use crate::host::BevyHost;
use crate::with_kernel;

/// The pointer is over this node.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hovered;

/// The pointer is held down on this node.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Pressed;

/// Tagging a node from Bevy's events, for whatever a call site wires
/// up.
pub trait TagExt<Theme: Send + Sync + 'static> {
    /// This element's own node.
    fn id(&self) -> Entity;

    /// Not for hand-written code.
    #[doc(hidden)]
    fn tag_world(&mut self) -> &mut World;

    /// Tag this node with `tag` whenever `V` fires on it.
    fn set_tag_on<V: EntityEvent, T: Tag>(
        &mut self,
        tag: T,
    ) -> &mut Self {
        let node = self.id();
        let world = self.tag_world();
        replace::<V, T>(
            world,
            node,
            Observer::new(move |_: On<V>, mut commands: Commands| {
                commands.queue(move |world: &mut World| {
                    with_kernel::<Theme>(world, |kernel, _| {
                        kernel.set_tag(node, tag);
                    });
                });
            }),
        );
        self
    }

    /// Drop this node's tag of type `T` whenever `V` fires on it.
    fn unset_tag_on<V: EntityEvent, T: Tag>(&mut self) -> &mut Self {
        let node = self.id();
        let world = self.tag_world();
        replace::<V, T>(
            world,
            node,
            Observer::new(move |_: On<V>, mut commands: Commands| {
                commands.queue(move |world: &mut World| {
                    with_kernel::<Theme>(world, |kernel, _| {
                        kernel.unset_tag::<T>(node);
                    });
                });
            }),
        );
        self
    }

    /// The usual pointer wiring: [`Hovered`] while the pointer is
    /// over, [`Pressed`] while it is held.
    ///
    /// `Release` and `Cancel` drop only [`Pressed`]; a release still
    /// over the node leaves [`Hovered`] set, so a field falls back to
    /// its hover line rather than to its base.
    fn pointer_tags(&mut self) -> &mut Self {
        self.set_tag_on::<Pointer<Over>, _>(Hovered)
            .unset_tag_on::<Pointer<Out>, Hovered>()
            .set_tag_on::<Pointer<Press>, _>(Pressed)
            .unset_tag_on::<Pointer<Release>, Pressed>()
            .unset_tag_on::<Pointer<Cancel>, Pressed>()
    }
}

impl<Theme: Send + Sync + 'static, E: Element<BevyHost<Theme>>>
    TagExt<Theme> for ElementMut<'_, '_, BevyHost<Theme>, E>
{
    fn id(&self) -> Entity {
        ElementMut::id(self)
    }

    fn tag_world(&mut self) -> &mut World {
        self.ui.world
    }
}

impl<Theme: Send + Sync + 'static, E: Element<BevyHost<Theme>>>
    TagExt<Theme> for Build<'_, BevyHost<Theme>, E>
{
    fn id(&self) -> Entity {
        Build::id(self)
    }

    fn tag_world(&mut self) -> &mut World {
        self.world
    }
}

/// The observer currently answering `V` with a change to `T` on this
/// node. Keyed on both, so two tags driven by one event do not evict
/// each other.
#[derive(Component)]
struct Tagging<V, T>(Entity, PhantomData<fn() -> (V, T)>);

/// Point `observer` at `node`, dropping whatever `(V, T)` was
/// watching it before.
fn replace<V: EntityEvent, T: Tag>(
    world: &mut World,
    node: Entity,
    observer: Observer,
) {
    // `EntityWorldMut::observe` hands back the entity it watches, not
    // the observer it made, so there'd be no way to find this one
    // again to despawn it.
    if let Some(&Tagging::<V, T>(old, _)) =
        world.get::<Tagging<V, T>>(node)
    {
        world.despawn(old);
    }

    let observer = world.spawn(observer.with_entity(node)).id();

    world
        .entity_mut(node)
        .insert(Tagging::<V, T>(observer, PhantomData));
}
