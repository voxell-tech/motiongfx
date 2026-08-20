//! Inspectors, as composers rather than elements.
//!
//! What an inspector is handed (an entity, a component's type, a
//! resource's type) decides what its subtree *is*, not what a node
//! looks like. There is no patch for "build something else instead",
//! so these read their input once, while building, exactly a
//! [`Composer`]'s window.
//!
//! Each is empty when what it points at is not there. A missing
//! component and an inspector pointed nowhere read the same.

use std::any::TypeId;

use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::std_traits::ReflectDefault;
use bevy::ui_widgets::{Activate, ActivateOnPress, MenuButton};

use bevy_fynix::EntityExt;
use fynix_mock::composer::Composer;
use fynix_mock::records::{BuildFn, ChangedFn};
use fynix_mock::ui::ElementHandle;
use fynix_mock::{elem, val};

use super::{
    Dropdown, DropdownItem, DropdownItemCursor, DropdownList,
    DropdownMenu, Frame, Icon, Label, TintButton,
};
use crate::icons;
use crate::inspector::{
    Field, InspectorFields, ReflectInspectable, Section,
    inspect_value, is_single_value,
};
use crate::motion::MotionExt;
use crate::reactive::{BevyHost, BevyUi, value_changed};
use crate::theme::EditorTheme;

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

/// Every component of one entity the inspector can read: those with
/// fields under a collapsible header of their own, and those holding
/// a single value on one row.
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
                let field = Field::new(entity, component);

                if is_single_value(ui.world, &field) {
                    single(ui, name, field);
                    continue;
                }

                ui.compose(Section {
                    name: name.to_string(),
                    body: move |ui: &mut BevyUi| {
                        ui.compose(ComponentInspector {
                            entity,
                            component,
                        });
                    },
                });
            }

            ui.compose(AddComponent { entity });
        })
    }
}

/// The menu that adds a component to `entity`.
///
/// Rebuilt alongside [`EntityInspector`], on the same signal: what it
/// offers is exactly what that signal tracks.
struct AddComponent {
    entity: Entity,
}

impl Composer<BevyHost> for AddComponent {
    type Element = DropdownMenu;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, DropdownMenu> {
        let entity = self.entity;
        let theme = ui.world.resource::<EditorTheme>().clone();
        let options = addable(ui.world, entity);
        let width = Dropdown::width_for(
            &options
                .iter()
                .map(|(_, name)| name.to_string())
                .collect::<Vec<_>>(),
            12.0,
        );

        ui.elem(elem!(DropdownMenu))
            .with(move |ui| {
                ui.elem(elem!(
                    !TintButton::default(),
                    icon = val!(
                        Icon,
                        image = icons::PLUS,
                        color = theme.text_muted
                    )
                ))
                .insert((MenuButton, ActivateOnPress));

                ui.elem(elem!(DropdownList, width = width)).with(
                    move |ui| {
                        // A menu popup only opens with a focusable
                        // child, so an empty list says why it's
                        // empty instead of showing nothing.
                        if options.is_empty() {
                            ui.elem(elem!(
                                DropdownItem,
                                label = val!(
                                    Label,
                                    text = "Nothing left to add"
                                        .to_string(),
                                    color = Some(theme.text_muted)
                                )
                            ));
                            return;
                        }

                        for (component, name) in options {
                            add_component_item(
                                ui, &theme, entity, component, name,
                            );
                        }
                    },
                );
            })
            .handle()
    }
}

/// One entry in [`AddComponent`]'s list. Picking it inserts the
/// component and closes the list.
fn add_component_item(
    ui: &mut BevyUi,
    theme: &EditorTheme,
    entity: Entity,
    component: TypeId,
    name: &str,
) {
    ui.elem(elem!(
        DropdownItem,
        label = val!(
            Label,
            text = name.to_string(),
            wrap = false,
            color = Some(theme.text_primary)
        )
    ))
    .lit(|item| item.fill(), theme.hover_overlay, theme.hover_overlay)
    .observe(move |_: On<Activate>, mut commands: Commands| {
        commands.queue(move |world: &mut World| {
            add_component(world, entity, component);
        });
    });
}

/// Every [`register_inspectable`](
/// crate::inspector::InspectAppExt::register_inspectable) type
/// `entity` does not already carry, with a registered
/// [`ReflectDefault`] to construct one with.
///
/// Sorted by name, for the same reason [`inspectable`] is.
fn addable(
    world: &World,
    entity: Entity,
) -> Vec<(TypeId, &'static str)> {
    let Ok(entity_ref) = world.get_entity(entity) else {
        return Vec::new();
    };

    let registry = world.resource::<AppTypeRegistry>().read();
    let mut out: Vec<(TypeId, &'static str)> = registry
        .iter()
        .filter(|registration| {
            registration.data::<ReflectInspectable>().is_some()
                && registration.data::<ReflectDefault>().is_some()
        })
        .filter_map(|registration| {
            let reflect_component =
                registration.data::<ReflectComponent>()?;
            if reflect_component.contains(entity_ref) {
                return None;
            }
            Some((
                registration.type_id(),
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

/// Inserts `component`'s default value onto `entity`. Does nothing
/// if the entity despawned or the component arrived some other way
/// before this runs.
fn add_component(
    world: &mut World,
    entity: Entity,
    component: TypeId,
) {
    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry = registry.read();

    let Some(registration) = registry.get(component) else {
        return;
    };
    let (Some(default), Some(reflect_component)) = (
        registration.data::<ReflectDefault>(),
        registration.data::<ReflectComponent>(),
    ) else {
        return;
    };
    let Ok(mut entity) = world.get_entity_mut(entity) else {
        return;
    };
    if reflect_component.contains(entity.as_readonly()) {
        return;
    }

    let value = default.default();
    reflect_component.insert(
        &mut entity,
        value.as_partial_reflect(),
        &registry,
    );
}

/// A whole component on one row, named where a group of fields would
/// have been headed.
fn single(ui: &mut BevyUi, name: &str, field: Field) {
    let theme = ui.world.resource::<EditorTheme>().clone();
    let name = name.to_string();

    ui.elem(elem!(
        Frame,
        width = percent(100),
        direction = FlexDirection::Row,
        justify = JustifyContent::SpaceBetween,
        align = AlignItems::Center,
        column_gap = px(8),
        padding = UiRect::vertical(px(3))
    ))
    .with(move |ui| {
        ui.elem(elem!(
            Label,
            text = name,
            color = Some(theme.text_primary),
            bold = true
        ));
        // Straight from the field: a `ComponentInspector` fills the
        // width it is given, leaving nothing for the name beside it.
        inspect_value(ui, &field);
    });
}

/// The entity bevy is currently keeping `resource` on.
fn resource_entity(
    world: &World,
    resource: TypeId,
) -> Option<Entity> {
    let id = world.components().get_id(resource)?;
    world.resource_entities().get(id)
}

/// Fires when the entity holding `resource` changes, and on the
/// first poll.
///
/// Only moves when the resource is removed and re-inserted: a
/// different entity means the subtree should rebuild, not have its
/// old bindings quietly re-pointed.
fn entity_changed(
    resource: TypeId,
) -> impl FnMut(&World, Entity) -> bool {
    value_changed(move |world, _| resource_entity(world, resource))
}

/// Fires when `entity`'s set of inspectable components changes, and
/// on the first poll.
///
/// A component's *value* is each section's own business. This only
/// rebuilds when one is added, removed, or the entity goes.
fn components_changed(
    entity: Entity,
) -> impl FnMut(&World, Entity) -> bool {
    value_changed(move |world, _| inspectable(world, entity))
}

/// Every component on `entity` the inspector can reach and shows, by
/// type and the short name its section is headed with.
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
    // Collected before the registry is read: both borrow the world.
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
            // Opt-in - see InspectAppExt::register_inspectable.
            registration.data::<ReflectInspectable>()?;
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

/// Fires when `field`'s component appears or disappears, and on the
/// first poll.
///
/// Presence only. What the component holds is
/// [`InspectorFields`]' own business, with its own watcher for that.
fn presence_changed(
    field: Field,
) -> impl FnMut(&World, Entity) -> bool {
    value_changed(move |world, _| field.exists(world))
}
