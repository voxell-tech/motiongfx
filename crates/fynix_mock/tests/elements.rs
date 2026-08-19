//! Drives an element tree: build it, patch a field two elements down,
//! tear it down.

mod common;

use common::{Backend, Label, LabelCursor, World};
use fynix_mock::element::{Element, ElementVisual, Fields};
use fynix_mock::host::Host;
use fynix_mock::lenz::FieldPath;
use fynix_mock::ui::Records;
use fynix_mock::ui::Ui;

#[derive(Element)]
pub struct Icon {
    #[default('+')]
    pub glyph: char,
}

/// Plain data, not an element: no node of its own, so `Button` draws
/// it.
#[derive(Element)]
pub struct Border {
    pub width: u32,
    pub radius: u32,
}

#[derive(Element)]
pub struct Button {
    #[elem(child)]
    pub label: Label,
    /// Present by default, so the tests that want one say nothing and
    /// the one that wants none clears it.
    #[elem(child)]
    #[default(..)]
    pub icon: Option<Icon>,
    #[default(4)]
    pub padding: u32,
    pub border: Border,
}

impl ElementVisual<Backend> for Icon {
    fn build_fields(&self, ui: &mut Ui<'_, common::Backend>) {
        let node = ui.parent();
        let world = &mut *ui.world;

        world.node(node).glyph = self.glyph;
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: usize,
        field: IconField,
    ) {
        match field {
            IconField::Glyph => world.node(node).glyph = self.glyph,
        }
    }
}

impl ElementVisual<Backend> for Button {
    /// Only what `Button` draws. Nothing here mentions `label` or
    /// `icon`: the derive builds and patches those.
    fn build_fields(&self, ui: &mut Ui<'_, common::Backend>) {
        let node = ui.parent();
        let world = &mut *ui.world;

        world.node(node).padding = self.padding;
        world.node(node).border_width = self.border.width;
        world.node(node).border_radius = self.border.radius;
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: usize,
        field: ButtonField,
    ) {
        match field {
            ButtonField::Padding => {
                world.node(node).padding = self.padding
            }
            // Plain data, so it goes on whole. Only elements are
            // worth reaching into one field at a time.
            ButtonField::Border => {
                world.node(node).border_width = self.border.width;
                world.node(node).border_radius = self.border.radius;
            }
        }
    }
}

#[test]
fn build_writes_the_element_and_its_children() {
    let (mut world, parent) = World::with_root();
    let mut records = Records::default();
    let button = Button::default();

    let node = button.build(&mut world, parent, &mut records, &());

    assert_eq!(world.get(node).padding, 4);

    let label =
        records.store().get(node, button_path::label::id()).unwrap();
    let icon =
        records.store().get(node, button_path::icon::id()).unwrap();
    assert_eq!(world.get(label).text, "Label");
    assert_eq!(world.get(label).size, 13);
    assert_eq!(world.get(icon).glyph, '+');
}

#[test]
fn absent_child_leaves_no_entry() {
    let (mut world, parent) = World::with_root();
    let mut records = Records::default();
    let button = Button {
        icon: None,
        ..Button::default()
    };

    let node = button.build(&mut world, parent, &mut records, &());

    assert!(
        records.store().get(node, button_path::icon::id()).is_none()
    );
    assert!(
        records
            .store()
            .get(node, button_path::label::id())
            .is_some()
    );
}

#[test]
fn path_into_a_child_is_patched_by_that_child() {
    let (mut world, parent) = World::with_root();
    let mut records = Records::default();
    let mut button = Button::default();
    let node = button.build(&mut world, parent, &mut records, &());
    let label =
        records.store().get(node, button_path::label::id()).unwrap();

    // Write through the lens, then patch the field it walked to.
    let path = Button::cursor().label().text();
    *(path.accessor().get_mut)(&mut button).unwrap() = "Saved".into();
    button.patch(&mut world, node, &path.hops(), records.store_mut());

    assert_eq!(world.get(label).text, "Saved");
    assert_eq!(world.get(label).size, 13);
    assert_eq!(world.get(node).padding, 4);
}

#[test]
fn path_into_plain_data_is_finished_by_its_owner() {
    let (mut world, parent) = World::with_root();
    let mut records = Records::default();
    let mut button = Button::default();
    let node = button.build(&mut world, parent, &mut records, &());

    let path = Button::cursor().border().width();
    *(path.accessor().get_mut)(&mut button).unwrap() = 2;
    button.patch(&mut world, node, &path.hops(), records.store_mut());

    assert_eq!(world.get(node).border_width, 2);
    assert_eq!(world.get(node).border_radius, 0);
}

#[test]
fn elem_field_is_not_one_of_our_own() {
    // `label` and `icon` are elements, so they never reach `Button`'s
    // own dispatch: nothing there can name them.
    assert!(Button::field(button_path::label::id()).is_none());
    assert!(Button::field(button_path::icon::id()).is_none());

    assert_eq!(
        Button::field(button_path::padding::id()),
        Some(ButtonField::Padding)
    );
}

#[test]
fn patching_an_unnamed_field_changes_nothing() {
    let (mut world, parent) = World::with_root();
    let mut records = Records::default();
    let button = Button::default();
    let node = button.build(&mut world, parent, &mut records, &());

    // A real path, but to a field of something else entirely.
    let path = Label::cursor().size().hops();
    button.patch(&mut world, node, &path, records.store_mut());

    assert_eq!(world.get(node).padding, 4);
}

#[test]
fn despawn_takes_the_children_and_their_entries() {
    let (mut world, parent) = World::with_root();
    let mut records = Records::default();
    let button = Button::default();
    let node = button.build(&mut world, parent, &mut records, &());
    let label =
        records.store().get(node, button_path::label::id()).unwrap();
    let icon =
        records.store().get(node, button_path::icon::id()).unwrap();

    button.despawn(&mut world, node, records.store_mut());

    assert!(!Backend::exists(&world, node));
    assert!(!Backend::exists(&world, label));
    assert!(!Backend::exists(&world, icon));
    assert!(records.store().is_empty());
    assert!(Backend::exists(&world, parent));
}

#[test]
fn pruning_drops_what_the_app_despawned() {
    let (mut world, parent) = World::with_root();
    let mut records = Records::default();
    let button = Button::default();
    let node = button.build(&mut world, parent, &mut records, &());

    assert_eq!(records.store().len(), 2);

    // Behind our back, so nothing cleared the store as it went.
    Backend::despawn(&mut world, node);
    records.store_mut().prune(&world);

    assert!(records.store().is_empty());
}

#[test]
fn walk_names_one_id_per_hop() {
    assert_eq!(Button::cursor().padding().hops().len(), 1);
    assert_eq!(Button::cursor().label().text().hops().len(), 2);
    assert_eq!(
        Button::cursor().label().hops()[0],
        button_path::label::id()
    );
}
