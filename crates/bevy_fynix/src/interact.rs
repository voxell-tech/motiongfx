//! Wiring a lane to a pointer event, which is Bevy's own concern and
//! not fynix's: [`Fynix::aim`] takes a node, a field, and a target,
//! and nothing about when to call it. [`Aiming`] is what decides when,
//! for whichever [`EntityEvent`] a call site names - there is no
//! closed vocabulary of "interactions" here, only whatever event type
//! is asked for.

use core::marker::PhantomData;
use std::sync::Arc;

use bevy_ecs::prelude::*;
use fynix_mock::Fynix;
use fynix_mock::element::Element;
use fynix_mock::lenz::{Cursor, FieldPath, Identity};
use fynix_mock::ui::ElementMut;

use crate::host::BevyHost;
use crate::with_kernel;

/// One queued aim: point a field somewhere on the kernel, given the
/// node it belongs to.
type Aim = Box<dyn Fn(&mut Fynix<BevyHost>, Entity) + Send + Sync>;

/// Aims queued for one event type on one node, until they are dropped
/// onto a single observer.
///
/// A statement's worth of `.aim(...)` calls, so
/// `label.on::<V>().aim(a).aim(b);` registers one observer that runs
/// both, rather than one observer per field.
///
/// `watch` and `aim` differ for a lane on a `#[elem]` child: the event
/// has to come from the child's own hit area, but the lane lives
/// keyed on the owner, which is where every `.transition()` and
/// `.bind()` on it puts things.
pub struct Aiming<'w, E: 'static, V: EntityEvent> {
    aim: Entity,
    watch: Entity,
    world: &'w mut World,
    aims: Vec<Aim>,
    marker: PhantomData<fn() -> (E, V)>,
}

impl<E: 'static, V: EntityEvent> Aiming<'_, E, V> {
    /// Point `field` at `target` whenever the event this was opened
    /// for fires on this node, or release it with `None`.
    ///
    /// The trigger half of
    /// [`ElementMut::transition`](fynix_mock::ui::ElementMut::transition):
    /// aiming a field with no lane does nothing.
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

impl<E: 'static, V: EntityEvent> Drop for Aiming<'_, E, V> {
    fn drop(&mut self) {
        let aims = core::mem::take(&mut self.aims);
        if aims.is_empty() {
            return;
        }

        watch::<V>(
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
pub trait OnExt<E: Element<BevyHost>> {
    /// Open a group of aims that fire together whenever `V` happens to
    /// this node. Ends, and registers as one observer, at the `;`.
    fn on<V: EntityEvent>(&mut self) -> Aiming<'_, E, V>;

    /// The same, but watching `child` rather than this node, for a
    /// lane on a `#[elem]` field whose own hit area should be what
    /// reacts, found with [`ElementMut::child`].
    fn on_entity<V: EntityEvent>(
        &mut self,
        child: Entity,
    ) -> Aiming<'_, E, V>;
}

impl<E: Element<BevyHost>> OnExt<E>
    for ElementMut<'_, '_, BevyHost, E>
{
    fn on<V: EntityEvent>(&mut self) -> Aiming<'_, E, V> {
        let node = self.id();
        self.on_entity(node)
    }

    fn on_entity<V: EntityEvent>(
        &mut self,
        entity: Entity,
    ) -> Aiming<'_, E, V> {
        Aiming {
            aim: self.id(),
            watch: entity,
            world: self.ui.world,
            aims: Vec::new(),
            marker: PhantomData,
        }
    }
}

/// Run `aim` whenever `V` fires on `watch`, naming `aim_node` as what
/// it moves.
///
/// Queued rather than run there and then, because a flush owns the
/// kernel while it runs and an observer cannot know it isn't inside
/// one.
fn watch<V: EntityEvent>(
    world: &mut World,
    watch: Entity,
    aim_node: Entity,
    aim: impl Fn(&mut Fynix<BevyHost>, Entity) + Send + Sync + 'static,
) {
    let aim = Arc::new(aim);

    world.entity_mut(watch).observe(
        move |_: On<V>, mut commands: Commands| {
            let aim = Arc::clone(&aim);

            commands.queue(move |world: &mut World| {
                with_kernel(world, |kernel, _| aim(kernel, aim_node));
            });
        },
    );
}
