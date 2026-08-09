//! A generic element: its paths carry the arguments, so two of them
//! never share an id.

mod common;

use common::{Label, LabelCursor, World};
use fynix_mock::element::{Element, ElementVisual, Fields};
use fynix_mock::lenz::Lenz;
use fynix_mock::store::Store;

/// `Default`, so an element generic over its look has one too.
pub trait Look: Default + 'static {
    fn glyph(&self) -> char;
}

#[derive(Default)]
pub struct Dark;

impl Look for Dark {
    fn glyph(&self) -> char {
        '*'
    }
}

#[derive(Element, Default, Lenz)]
pub struct Themed<L: Look> {
    #[elem]
    pub label: Label,
    pub look: L,
}

impl<L: Look> ElementVisual<common::Backend> for Themed<L> {
    fn build_fields(&self, world: &mut World, node: usize) {
        world.node(node).glyph = self.look.glyph();
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: usize,
        field: ThemedField,
    ) {
        match field {
            ThemedField::Look => {
                world.node(node).glyph = self.look.glyph()
            }
        }
    }
}

#[test]
fn generic_element_builds_its_children() {
    let (mut world, parent) = World::with_root();
    let mut store = Store::new();
    let themed = Themed::<Dark>::default();

    let node = themed.build(&mut world, parent, &mut store);

    assert_eq!(world.get(node).glyph, '*');

    let label = Themed::<Dark>::cursor().label().hops();
    let label = store.get(node, label[0]).unwrap();
    assert_eq!(world.get(label).text, "Label");
}

#[test]
fn generic_element_patches_through_its_child() {
    let (mut world, parent) = World::with_root();
    let mut store = Store::new();
    let mut themed = Themed::<Dark>::default();
    let node = themed.build(&mut world, parent, &mut store);

    let path = Themed::<Dark>::cursor().label().text();
    let ids = path.hops();
    let label = store.get(node, ids[0]).unwrap();

    *(path.accessor().get_mut)(&mut themed).unwrap() = "Saved".into();
    themed.patch(&mut world, node, &ids, &mut store);

    assert_eq!(world.get(label).text, "Saved");
}

#[test]
fn generic_elem_field_is_still_left_out_of_the_enum() {
    let label = Themed::<Dark>::cursor().label().hops();
    assert!(Themed::<Dark>::field(label[0]).is_none());

    let look = Themed::<Dark>::field_id(ThemedField::Look);
    assert_eq!(Themed::<Dark>::field(look), Some(ThemedField::Look));
}
