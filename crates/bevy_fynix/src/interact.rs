//! Wiring a lane to a pointer event, Bevy's own concern, not fynix's.
//! [`Fynix::aim`] takes a node, a field, and a target, with no notion
//! of when to call it. [`Aiming`] decides when, for whichever
//! [`EntityEvent`] a call site names.

use core::marker::PhantomData;
use std::sync::Arc;

use bevy_ecs::prelude::*;
use fynix::Fynix;
use fynix::element::Element;
use fynix::lenz::{Cursor, FieldPath, Identity};
use fynix::ui::{Build, ElementMut};

use crate::host::BevyHost;
use crate::with_kernel;

/// One queued aim: point a field somewhere on the kernel, given the
/// node it belongs to.
type Aim<Theme> =
    Box<dyn Fn(&mut Fynix<BevyHost<Theme>>, Entity) + Send + Sync>;

/// Aims queued for one event type on one node, until they are dropped
/// onto a single observer.
///
/// `label.on::<V>().aim(a).aim(b);` registers a single observer for
/// both.
///
/// `watch` and `aim` differ for a `#[elem(child)]` lane: the event
/// comes from the child's own hit area, but the lane is keyed on the
/// owner.
pub struct Aiming<
    'w,
    E: 'static,
    Theme: Send + Sync + 'static,
    V: EntityEvent,
> {
    aim: Entity,
    watch: Entity,
    world: &'w mut World,
    aims: Vec<Aim<Theme>>,
    marker: PhantomData<fn() -> (E, V)>,
}

impl<E: 'static, Theme: Send + Sync + 'static, V: EntityEvent>
    Aiming<'_, E, Theme, V>
{
    /// Point `field` at `target` whenever the event this was opened
    /// for fires on this node, or release it with `None`. Aiming a
    /// field with no lane does nothing.
    pub fn aim<P>(
        mut self,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        target: Option<P::Target>,
    ) -> Self
    where
        P: FieldPath<Source = E>,
        P::Target: Clone + Send + Sync,
    {
        self.aims.push(Box::new(move |kernel, node| {
            kernel.aim(node, field, target.clone());
        }));
        self
    }
}

impl<E: 'static, Theme: Send + Sync + 'static, V: EntityEvent> Drop
    for Aiming<'_, E, Theme, V>
{
    fn drop(&mut self) {
        let aims = core::mem::take(&mut self.aims);
        if aims.is_empty() {
            return;
        }

        watch::<Theme, V>(
            self.world,
            self.watch,
            self.aim,
            move |kernel, node| {
                for aim in &aims {
                    aim(kernel, node);
                }
            },
        );
    }
}

/// What Bevy wants on a node that the element itself has no say in.
///
/// `on` is just `on_entity` aimed at this node's own id. Only
/// `on_entity` differs between `ElementMut` and `Build`.
pub trait OnExt<
    E: Element<BevyHost<Theme>>,
    Theme: Send + Sync + 'static,
>
{
    /// Open a group of aims that fire together whenever `V` happens to
    /// this node. Ends, and registers as one observer, at the `;`.
    fn on<V: EntityEvent>(&mut self) -> Aiming<'_, E, Theme, V> {
        let node = self.id();
        self.on_entity(node)
    }

    /// This element's own node.
    fn id(&self) -> Entity;

    /// The same, but watching `child`: for a `#[elem(child)]` field
    /// whose own hit area should react.
    fn on_entity<V: EntityEvent>(
        &mut self,
        child: Entity,
    ) -> Aiming<'_, E, Theme, V>;
}

impl<Theme: Send + Sync + 'static, E: Element<BevyHost<Theme>>>
    OnExt<E, Theme> for ElementMut<'_, '_, BevyHost<Theme>, E>
{
    fn id(&self) -> Entity {
        ElementMut::id(self)
    }

    fn on_entity<V: EntityEvent>(
        &mut self,
        entity: Entity,
    ) -> Aiming<'_, E, Theme, V> {
        Aiming {
            aim: self.id(),
            watch: entity,
            world: self.ui.world,
            aims: Vec::new(),
            marker: PhantomData,
        }
    }
}

impl<Theme: Send + Sync + 'static, E: Element<BevyHost<Theme>>>
    OnExt<E, Theme> for Build<'_, BevyHost<Theme>, E>
{
    fn id(&self) -> Entity {
        Build::id(self)
    }

    fn on_entity<V: EntityEvent>(
        &mut self,
        entity: Entity,
    ) -> Aiming<'_, E, Theme, V> {
        Aiming {
            aim: self.id(),
            watch: entity,
            world: self.world,
            aims: Vec::new(),
            marker: PhantomData,
        }
    }
}

/// The observer currently watching `V` on this node. Lets a rewire
/// despawn it before spawning a replacement, since
/// `EntityWorldMut::observe` always adds, never replaces.
#[derive(Component)]
struct Watching<V>(Entity, PhantomData<fn() -> V>);

/// Run `aim` whenever `V` fires on `watch`, naming `aim_node` as what
/// it moves.
///
/// Queued, not run immediately: a flush owns the kernel while it
/// runs, and an observer cannot know it isn't inside one.
fn watch<Theme: Send + Sync + 'static, V: EntityEvent>(
    world: &mut World,
    watch: Entity,
    aim_node: Entity,
    aim: impl Fn(&mut Fynix<BevyHost<Theme>>, Entity)
    + Send
    + Sync
    + 'static,
) {
    let aim = Arc::new(aim);

    // `EntityWorldMut::observe` hands back the entity it watches, not
    // the observer it made, so there'd be no way to find this one
    // again to despawn it.
    if let Some(&Watching(old, _)) = world.get::<Watching<V>>(watch) {
        world.despawn(old);
    }

    let observer = world
        .spawn(
            Observer::new(move |_: On<V>, mut commands: Commands| {
                let aim = Arc::clone(&aim);

                commands.queue(move |world: &mut World| {
                    with_kernel::<Theme>(world, |kernel, _| {
                        aim(kernel, aim_node)
                    });
                });
            })
            .with_entity(watch),
        )
        .id();

    world
        .entity_mut(watch)
        .insert(Watching::<V>(observer, PhantomData));
}
