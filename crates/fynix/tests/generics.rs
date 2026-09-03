//! A generic element: its paths carry the arguments, so two of them
//! never share an id.

mod common;

use core::marker::PhantomData;

use common::{FynixHost, Label, LabelCursor, World};
use fynix::element::{Element, ElementBase, Fields, element};
use fynix::records::Records;
use fynix::ui::{FieldPatch, Patch};

/// `Default`, so an element generic over its look starts from one.
/// `Send`/`Sync` too, since `Themed<L>` is an `Element`.
pub trait Look: Default + Send + Sync + 'static {
    fn glyph(&self) -> char;
}

#[derive(Default)]
pub struct Dark;

impl Look for Dark {
    fn glyph(&self) -> char {
        '*'
    }
}

#[element]
pub struct Themed<L: Look> {
    #[elem(child)]
    pub label: Label,
    #[elem(patch = WriteLook::<L>)]
    pub look: L,
}

pub struct WriteLook<L>(PhantomData<fn() -> L>);

impl<L: Look> FieldPatch<FynixHost> for WriteLook<L> {
    type Target = L;

    fn patch(patch: &mut Patch<FynixHost>, look: &L) {
        let node = patch.id();
        patch.world.node(node).glyph = look.glyph();
    }
}

#[test]
fn generic_element_builds_its_children() {
    let (mut world, parent) = World::with_root();
    let mut records = Records::default();
    let themed = Themed::<Dark>::base(&());

    let node = themed.build(&mut world, parent, &mut records, &());

    assert_eq!(world.get(node).glyph, '*');

    let label = Themed::<Dark>::cursor().label().hops();
    let label = records.store().get(node, label[0]).unwrap();
    assert_eq!(world.get(label).text, "Label");
}

#[test]
fn generic_element_patches_through_its_child() {
    let (mut world, parent) = World::with_root();
    let mut records = Records::default();
    let mut themed = Themed::<Dark>::base(&());
    let node = themed.build(&mut world, parent, &mut records, &());

    let path = Themed::<Dark>::cursor().label().text();
    let ids = path.hops();
    let label = records.store().get(node, ids[0]).unwrap();

    *path.accessor().get_mut(&mut themed).unwrap() = "Saved".into();
    themed.patch(&mut world, node, &ids, records.store_mut(), &());

    assert_eq!(world.get(label).text, "Saved");
}

#[test]
fn generic_elem_field_is_still_left_out_of_the_enum() {
    let label = Themed::<Dark>::cursor().label().hops();
    assert!(Themed::<Dark>::field(label[0]).is_none());

    let look = Themed::<Dark>::field_id(ThemedField::Look);
    assert_eq!(Themed::<Dark>::field(look), Some(ThemedField::Look));
}
