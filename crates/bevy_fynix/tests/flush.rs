//! An element built into a real `World`, through the plugin.

use bevy_app::prelude::*;
use bevy_ecs::hierarchy::Children;
use bevy_ecs::prelude::*;
use bevy_fynix::host::BevyHost;
use bevy_fynix::{FynixPlugin, watch_root};
use bevy_ui::Node;
use fynix_mock::OverrideDefault;
use fynix_mock::elem;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

/// What the element writes. A real one would write `bevy_ui`
/// components; this only has to be visible from a test.
#[derive(Component, Debug, PartialEq)]
struct Caption(String);

#[derive(OverrideDefault, Lenz, Element)]
pub struct Label {
    #[default(String::from("Label"))]
    pub text: String,
}

impl ElementVisual<BevyHost> for Label {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert(Caption(self.text.clone()));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: LabelField,
    ) {
        match field {
            LabelField::Text => {
                world
                    .entity_mut(node)
                    .insert(Caption(self.text.clone()));
            }
        }
    }
}

/// The one child a build put under `root`.
fn only_child(world: &World, root: Entity) -> Entity {
    let children = world.get::<Children>(root).expect("a child");
    assert_eq!(children.len(), 1, "one element was built");
    children[0]
}

fn app_with_root() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(FynixPlugin);

    let root = app.world_mut().spawn(Node::default()).id();
    (app, root)
}

#[test]
fn flush_builds_what_a_root_declares() {
    let (mut app, root) = app_with_root();

    watch_root(app.world_mut(), root, |ui| {
        ui.elem(elem!(Label, text = "Save"));
    });

    app.update();

    let world = app.world();
    let label = only_child(world, root);

    assert_eq!(world.get::<Caption>(label).unwrap().0, "Save");
    assert!(world.get::<Node>(label).is_some(), "a layout node");
}

#[test]
fn root_is_built_once() {
    let (mut app, root) = app_with_root();

    watch_root(app.world_mut(), root, |ui| {
        ui.elem(elem!(Label));
    });

    app.update();
    let first = only_child(app.world(), root);

    app.update();
    let again = only_child(app.world(), root);

    assert_eq!(first, again, "not rebuilt, so not respawned");
}

#[test]
fn binding_patches_the_node_it_built() {
    #[derive(Resource, Default)]
    struct Source {
        text: String,
        changed: bool,
    }

    let (mut app, root) = app_with_root();
    app.init_resource::<Source>();

    watch_root(app.world_mut(), root, |ui| {
        ui.elem(elem!(Label)).bind(
            |label| label.text(),
            |world: &World, _| world.resource::<Source>().changed,
            |world: &World, _| {
                world.resource::<Source>().text.clone()
            },
        );
    });

    app.update();
    let label = only_child(app.world(), root);
    assert_eq!(app.world().get::<Caption>(label).unwrap().0, "Label");

    {
        let mut source = app.world_mut().resource_mut::<Source>();
        source.text = "Saved".into();
        source.changed = true;
    }
    app.update();

    assert_eq!(only_child(app.world(), root), label, "the same node");
    assert_eq!(app.world().get::<Caption>(label).unwrap().0, "Saved");
}
