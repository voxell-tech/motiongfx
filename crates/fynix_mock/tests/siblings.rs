//! Two children of the same type. An id names the path, not the
//! type, so they cannot land on one store entry.

mod common;

use common::{Label, LabelCursor, World};
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::records::Records;
use fynix_mock::ui::{Build, Patch};

/// Two labels that already say which is which, so a test can tell one
/// node from the other without setting anything up.
#[derive(Element)]
pub struct Pair {
    #[elem(child)]
    #[default(text: String::from("up"), size: 1)]
    pub top: Label,
    #[elem(child)]
    #[default(text: String::from("down"), size: 2)]
    pub bottom: Label,
    #[default(7)]
    pub gap: u32,
}

impl ElementVisual<common::Backend> for Pair {
    fn build_fields(&self, build: &mut Build<common::Backend, Self>) {
        let node = build.id();
        let world = &mut *build.world;

        world.node(node).padding = self.gap;
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<common::Backend>,
        field: PairField,
    ) {
        let node = patch.id();
        let world = &mut *patch.world;

        match field {
            PairField::Gap => world.node(node).padding = self.gap,
        }
    }
}

#[test]
fn two_children_of_one_type_keep_separate_nodes() {
    let (mut world, parent) = World::with_root();
    let mut records = Records::default();
    let pair = Pair::default();

    let node = pair.build(&mut world, parent, &mut records, &());

    let top = Pair::cursor().top().hops();
    let bottom = Pair::cursor().bottom().hops();
    assert_ne!(top[0], bottom[0]);

    let top = records.store().get(node, top[0]).unwrap();
    let bottom = records.store().get(node, bottom[0]).unwrap();
    assert_ne!(top, bottom);
    assert_eq!(records.store().len(), 2);
    assert_eq!(world.get(top).text, "up");
    assert_eq!(world.get(bottom).text, "down");
}

#[test]
fn patching_one_of_a_pair_leaves_the_other() {
    let (mut world, parent) = World::with_root();
    let mut records = Records::default();
    let mut pair = Pair::default();
    let node = pair.build(&mut world, parent, &mut records, &());
    let top = records
        .store()
        .get(node, Pair::cursor().top().hops()[0])
        .unwrap();
    let bottom = records
        .store()
        .get(node, Pair::cursor().bottom().hops()[0])
        .unwrap();

    let path = Pair::cursor().bottom().text();
    *(path.accessor().get_mut)(&mut pair).unwrap() = "DOWN".into();
    pair.patch(
        &mut world,
        node,
        &path.hops(),
        records.store_mut(),
        &(),
    );

    assert_eq!(world.get(bottom).text, "DOWN");
    assert_eq!(world.get(top).text, "up");
}
