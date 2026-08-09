//! The first widget ported to fynix, built through the ECS.

use bevy::app::App;
use bevy::prelude::FontSize;
use bevy::prelude::{
    Children, Entity, Node, Resource, Text, TextFont, World,
};
use bevy_fynix::{FynixPlugin, watch_root};
use fynix_mock::elem;
use moxie_ui::fynix::{Label, LabelCursor};

/// What the label reads, so a binding has something to fire on.
#[derive(Resource, Default)]
struct Caption(String);

fn app_with_root() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(FynixPlugin).init_resource::<Caption>();

    let root = app.world_mut().spawn(Node::default()).id();
    (app, root)
}

/// The one child a build put under `root`.
fn only_child(world: &World, root: Entity) -> Entity {
    let children = world.get::<Children>(root).expect("a child");
    assert_eq!(children.len(), 1, "one element was built");
    children[0]
}

#[test]
fn label_writes_its_fields_as_components() {
    let (mut app, root) = app_with_root();

    watch_root(app.world_mut(), root, |ui| {
        ui.elem(elem!(!Label { text = "Save"; size = 20.0 }));
    });

    app.update();

    let world = app.world();
    let label = only_child(world, root);

    assert_eq!(world.get::<Text>(label).unwrap().0, "Save");
    assert_eq!(
        world.get::<TextFont>(label).unwrap().font_size,
        FontSize::Px(20.0)
    );
}

#[test]
fn bound_field_is_patched_without_a_rebuild() {
    let (mut app, root) = app_with_root();

    watch_root(app.world_mut(), root, |ui| {
        ui.elem(elem!(!Label)).bind(
            |label| label.text(),
            |world: &World, _| world.is_resource_changed::<Caption>(),
            |world: &World, _| world.resource::<Caption>().0.clone(),
        );
    });

    app.update();
    let label = only_child(app.world(), root);

    app.world_mut().resource_mut::<Caption>().0 = "Saved".into();
    app.update();

    assert_eq!(only_child(app.world(), root), label, "the same node");
    assert_eq!(app.world().get::<Text>(label).unwrap().0, "Saved");
}
