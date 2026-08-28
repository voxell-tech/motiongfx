//! What a flush does: run the watchers whose predicate fires, then
//! the bindings, then forget whatever the world no longer has.

mod common;

use common::{Backend, Label, LabelCursor, World};
use fynix::Fynix;
use fynix::WorldNodeRef;
use fynix::elem;
use fynix::host::Host;

/// Fires on the first call and never again, the way a bootstrap
/// build does. A stateful predicate consumes its own signal.
fn once() -> impl for<'w> FnMut(WorldNodeRef<'w, Backend>) -> bool
+ Send
+ Sync
+ 'static {
    let mut pending = true;
    move |_| core::mem::take(&mut pending)
}

/// The one child a watcher built under `root`.
fn only_child(world: &World, root: usize) -> usize {
    let children = Backend::children(world, root);
    assert_eq!(children.len(), 1, "one element was built");
    children[0]
}

#[test]
fn watch_builds_immediately_when_its_predicate_fires() {
    let (mut world, root) = World::with_root();
    let mut kernel = Fynix::new(());

    kernel.watch(
        root,
        once(),
        |ui| {
            ui.elem(elem!(Label));
        },
        &mut world,
    );

    let label = only_child(&world, root);
    assert_eq!(world.get(label).text, "Label");
    assert_eq!(world.get(label).size, 13);

    // `once()`'s first call already happened above, so a flush right
    // after does not rebuild.
    kernel.flush(&mut world);
    assert_eq!(only_child(&world, root), label, "not rebuilt");
}

#[test]
fn watch_builds_nothing_when_its_predicate_never_fires() {
    let (mut world, root) = World::with_root();
    let mut kernel = Fynix::new(());

    kernel.watch(
        root,
        |_| false,
        |ui| {
            ui.elem(elem!(Label));
        },
        &mut world,
    );

    assert!(Backend::children(&world, root).is_empty());

    kernel.flush(&mut world);

    assert!(Backend::children(&world, root).is_empty());
}

#[test]
fn binding_writes_and_patches_one_field() {
    let (mut world, root) = World::with_root();
    let mut kernel = Fynix::new(());

    kernel.watch(
        root,
        once(),
        |ui| {
            ui.elem(elem!(Label)).bind(
                |label| label.text(),
                |WorldNodeRef { world, .. }| world.source.changed,
                |WorldNodeRef { world, .. }| {
                    world.source.text.clone()
                },
            );
        },
        &mut world,
    );

    // Built immediately; the binding's own predicate has not fired,
    // so the element still holds what it was built with.
    let label = only_child(&world, root);
    assert_eq!(world.get(label).text, "Label");

    world.source.text = "Saved".into();
    world.source.changed = true;
    kernel.flush(&mut world);

    // The watcher did not fire this time, so nothing was rebuilt: the
    // node is the same one, with one field changed under it.
    assert_eq!(only_child(&world, root), label);
    assert_eq!(world.get(label).text, "Saved");
    assert_eq!(world.get(label).size, 13, "the rest is untouched");
}

#[test]
fn binding_stays_quiet_until_its_predicate_fires() {
    let (mut world, root) = World::with_root();
    let mut kernel = Fynix::new(());

    kernel.watch(
        root,
        once(),
        |ui| {
            ui.elem(elem!(Label)).bind(
                |label| label.text(),
                |WorldNodeRef { world, .. }| world.source.changed,
                |WorldNodeRef { world, .. }| {
                    world.source.text.clone()
                },
            );
        },
        &mut world,
    );

    let label = only_child(&world, root);

    // A new value, but nothing says so.
    world.source.text = "Saved".into();
    kernel.flush(&mut world);

    assert_eq!(world.get(label).text, "Label");
}

#[test]
fn rebuild_replaces_the_children() {
    let (mut world, root) = World::with_root();
    let mut kernel = Fynix::new(());

    // False at registration, so nothing builds until `changed` is
    // set below.
    kernel.watch(
        root,
        |WorldNodeRef { world, .. }: WorldNodeRef<Backend>| {
            world.source.changed
        },
        |ui| {
            ui.elem(elem!(Label));
        },
        &mut world,
    );
    assert!(Backend::children(&world, root).is_empty());

    world.source.changed = true;
    kernel.flush(&mut world);
    let first = only_child(&world, root);

    kernel.flush(&mut world);
    let second = only_child(&world, root);

    // The old subtree went, rather than a second one appearing
    // beside it.
    assert_ne!(first, second, "rebuilt, not added to");
    assert!(!Backend::exists(&world, first));
    assert_eq!(world.get(second).text, "Label");
}

#[test]
fn dead_node_is_not_patched() {
    let (mut world, root) = World::with_root();
    let mut kernel = Fynix::new(());

    kernel.watch(
        root,
        once(),
        |ui| {
            ui.elem(elem!(Label)).bind(
                |label| label.text(),
                |WorldNodeRef { world, .. }| world.source.changed,
                |WorldNodeRef { world, .. }| {
                    world.source.text.clone()
                },
            );
        },
        &mut world,
    );

    let label = only_child(&world, root);

    // Despawned behind the kernel's back. A binding kept against a
    // dead handle would patch a node the host has already freed.
    Backend::despawn(&mut world, label);

    world.source.text = "Saved".into();
    world.source.changed = true;
    kernel.flush(&mut world);

    assert!(!Backend::exists(&world, label));
    assert!(Backend::exists(&world, root));
}

#[test]
fn swept_node_takes_its_element_with_it() {
    let (mut world, root) = World::with_root();
    let mut kernel = Fynix::new(());

    // False at registration, so nothing builds until `changed` is
    // set below.
    kernel.watch(
        root,
        |WorldNodeRef { world, .. }: WorldNodeRef<Backend>| {
            world.source.changed
        },
        |ui| {
            ui.elem(elem!(Label));
        },
        &mut world,
    );

    world.source.changed = true;
    kernel.flush(&mut world);
    assert_eq!(kernel.element_len(), 1);

    // Rebuilt three times over. Each one clears the last, so the
    // elements the kernel holds must not pile up.
    kernel.flush(&mut world);
    kernel.flush(&mut world);
    kernel.flush(&mut world);
    assert_eq!(kernel.element_len(), 1, "one live element, not four");

    // Nothing to rebuild it now, so the last one goes too.
    world.source.changed = false;
    let last = only_child(&world, root);
    Backend::despawn(&mut world, last);
    kernel.flush(&mut world);

    assert_eq!(kernel.element_len(), 0);
}

#[test]
fn dead_root_takes_its_watcher_with_it() {
    let (mut world, root) = World::with_root();
    let mut kernel = Fynix::new(());

    // Under the root, so the watcher's own root can be despawned
    // without taking the whole world with it.
    let branch = Backend::spawn(&mut world, root);

    kernel.watch(
        branch,
        once(),
        |ui| {
            ui.elem(elem!(Label));
        },
        &mut world,
    );
    assert_eq!(kernel.watcher_len(), 1);

    kernel.flush(&mut world);
    assert_eq!(kernel.watcher_len(), 1, "its root is still there");

    Backend::despawn(&mut world, branch);
    kernel.flush(&mut world);

    assert_eq!(kernel.watcher_len(), 0, "nothing left to rebuild");
}

#[test]
fn dead_node_takes_its_bindings_with_it() {
    let (mut world, root) = World::with_root();
    let mut kernel = Fynix::new(());

    kernel.watch(
        root,
        once(),
        |ui| {
            ui.elem(elem!(Label)).bind(
                |label| label.text(),
                |WorldNodeRef { world, .. }| world.source.changed,
                |WorldNodeRef { world, .. }| {
                    world.source.text.clone()
                },
            );
        },
        &mut world,
    );

    let label = only_child(&world, root);
    assert_eq!(kernel.binding_len(), 1);

    Backend::despawn(&mut world, label);
    kernel.flush(&mut world);

    assert_eq!(kernel.binding_len(), 0, "the binding went with it");
}
