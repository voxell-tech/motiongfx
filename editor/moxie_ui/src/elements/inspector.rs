//! Inspectors, as composers rather than elements.
//!
//! What an inspector is handed - an entity, a component's type, a
//! resource's type - decides what its subtree *is*, not what a node
//! looks like. An element's field reaches the backend by being
//! patched onto a node that already exists, and there is no patch
//! that means "build something else instead", so these read their
//! inputs once, while building, which is exactly a
//! [`Composer`]'s window.
//!
//! Each is empty when what it points at is not there. Missing is not
//! an error worth a row of its own: an entity that lost a component
//! and an inspector that was never pointed anywhere read the same.

use std::any::TypeId;

use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;

use fynix_mock::composer::Composer;
use fynix_mock::elem;
use fynix_mock::ui::{BuildFn, ChangedFn, ElementHandle};

use super::Frame;
use crate::inspector::{Field, InspectorFields, Section};
use crate::reactive::{BevyHost, BevyUi};

/// One component of one entity.
pub struct ComponentInspector {
    pub entity: Entity,
    pub component: TypeId,
}

impl ComponentInspector {
    /// Names the component by type, for a call site that has one
    /// rather than a [`TypeId`].
    pub fn of<T: Component + Reflect>(entity: Entity) -> Self {
        Self {
            entity,
            component: TypeId::of::<T>(),
        }
    }
}

impl Composer<BevyHost> for ComponentInspector {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        let field = Field::new(self.entity, self.component);
        let built = field.clone();

        column(ui, px(4), presence_changed(field), move |ui| {
            if built.exists(ui.world) {
                ui.compose(InspectorFields {
                    root: built.clone(),
                });
            }
        })
    }
}

/// One resource.
///
/// Bevy parks each resource on an entity of its own, so once that
/// entity is in hand this is a [`ComponentInspector`] and nothing
/// below here knows the difference.
pub struct ResourceInspector {
    pub resource: TypeId,
}

impl ResourceInspector {
    /// Names the resource by type, for a call site that has one
    /// rather than a [`TypeId`].
    pub fn of<T: Resource + Reflect>() -> Self {
        Self {
            resource: TypeId::of::<T>(),
        }
    }
}

impl Composer<BevyHost> for ResourceInspector {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        let resource = self.resource;

        column(ui, px(4), entity_changed(resource), move |ui| {
            let Some(entity) = resource_entity(ui.world, resource)
            else {
                return;
            };
            ui.compose(InspectorFields {
                root: Field::new(entity, resource),
            });
        })
    }
}

/// Every component of one entity the inspector can read, each under
/// a collapsible header of its own.
pub struct EntityInspector {
    pub entity: Entity,
}

impl Composer<BevyHost> for EntityInspector {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        let entity = self.entity;

        column(ui, px(8), components_changed(entity), move |ui| {
            for (component, name) in inspectable(ui.world, entity) {
                // Headed by the whole component, at the empty path,
                // which the walk only ever hands groups *inside* -
                // so a component's fold and its groups' never
                // collide.
                let field = Field::new(entity, component);

                ui.compose(Section {
                    field,
                    name: name.to_string(),
                    body: move |ui: &mut BevyUi| {
                        ui.compose(ComponentInspector {
                            entity,
                            component,
                        });
                    },
                });
            }
        })
    }
}

/// The entity bevy is currently keeping `resource` on.
fn resource_entity(
    world: &World,
    resource: TypeId,
) -> Option<Entity> {
    let id = world.components().get_id(resource)?;
    world.resource_entities().get(id)
}

/// Fires when the entity holding `resource` changes, and on the first
/// poll.
///
/// It only moves when the resource is removed and re-inserted -
/// `insert_resource` reuses the entity while one is mapped and spawns
/// a fresh one otherwise - and that is a different value, which wants
/// the subtree rebuilt around it rather than the old bindings quietly
/// re-pointed.
fn entity_changed(
    resource: TypeId,
) -> impl FnMut(&World, Entity) -> bool {
    let mut seen: Option<Option<Entity>> = None;
    move |world, _| {
        let current = resource_entity(world, resource);
        let fired = seen != Some(current);
        seen = Some(current);
        fired
    }
}

/// Fires when `entity`'s set of inspectable components changes, and
/// on the first poll.
///
/// A component's *value* moves nothing here - that is each section's
/// own business - so this only rebuilds when one is added, removed,
/// or the entity itself goes.
fn components_changed(
    entity: Entity,
) -> impl FnMut(&World, Entity) -> bool {
    let mut seen: Option<Vec<(TypeId, &'static str)>> = None;
    move |world, _| {
        let current = inspectable(world, entity);
        let fired = seen.as_ref() != Some(&current);
        seen = Some(current);
        fired
    }
}

/// Every component on `entity` the inspector can reach, by type and
/// the short name its section is headed with.
///
/// Sorted by that name: an archetype lists what it holds in whatever
/// order it happens to, and a panel whose sections reshuffle when a
/// component is added is no use to read.
fn inspectable(
    world: &World,
    entity: Entity,
) -> Vec<(TypeId, &'static str)> {
    let Ok(components) = world.inspect_entity(entity) else {
        return Vec::new();
    };
    // Collected before the registry is read. Both borrow the same
    // world, so holding one across the other would only make the
    // lifetimes harder to follow than the walk is worth.
    let ids: Vec<TypeId> =
        components.filter_map(|info| info.type_id()).collect();

    let registry = world.resource::<AppTypeRegistry>().read();
    let mut out: Vec<(TypeId, &'static str)> = ids
        .into_iter()
        .filter_map(|id| {
            let registration = registry.get(id)?;
            // Without this there is no way to reach the value at
            // all, whatever its fields would have said.
            registration.data::<ReflectComponent>()?;
            Some((
                id,
                registration
                    .type_info()
                    .type_path_table()
                    .short_path(),
            ))
        })
        .collect();

    out.sort_by_key(|(_, name)| *name);
    out
}

/// The column an inspector fills, and the watcher that fills it.
///
/// `gap` is what separates its rows: wider between whole components
/// than between the fields of one.
fn column(
    ui: &mut BevyUi,
    gap: Val,
    changed: impl ChangedFn<BevyHost>,
    build: impl BuildFn<BevyHost>,
) -> ElementHandle<BevyHost, Frame> {
    ui.elem(elem!(
        Frame,
        width = percent(100),
        direction = FlexDirection::Column,
        row_gap = gap
    ))
    .watch(changed, build)
    .handle()
}

/// Fires when whether `field`'s component is there at all flips, and
/// on the first poll so the subtree starts out in step with the
/// world.
///
/// Presence only. What the component *holds* is
/// [`InspectorFields`]' own business, and it keeps a watcher for
/// that - rebuilding here on every shape change as well would only
/// throw that work away.
fn presence_changed(
    field: Field,
) -> impl FnMut(&World, Entity) -> bool {
    let mut seen: Option<bool> = None;
    move |world, _| {
        let present = field.exists(world);
        let fired = seen != Some(present);
        seen = Some(present);
        fired
    }
}
