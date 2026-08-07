//! Two children of the same type. An id names the path, not the
//! type, so they cannot land on one store entry.

mod common;

use common::{Label, LabelCursor, World};
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;
use fynix_mock::store::Store;

#[derive(Lenz, Element)]
pub struct Pair {
    #[elem]
    pub top: Label,
    #[elem]
    pub bottom: Label,
    pub gap: u32,
}

impl ElementVisual<common::Backend> for Pair {
    fn build_fields(&self, world: &mut World, node: usize) {
        world.node(node).padding = self.gap;
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: usize,
        field: PairField,
    ) {
        match field {
            PairField::Gap => world.node(node).padding = self.gap,
        }
    }
}

fn a_pair() -> Pair {
    Pair {
        top: Label {
            text: "up".into(),
            size: 1,
        },
        bottom: Label {
            text: "down".into(),
            size: 2,
        },
        gap: 7,
    }
}

#[test]
fn two_children_of_one_type_keep_separate_nodes() {
    let (mut world, parent) = World::with_root();
    let mut store = Store::new();
    let pair = a_pair();

    let node = pair.build(&mut world, parent, &mut store);

    let top = Pair::path().top().ids();
    let bottom = Pair::path().bottom().ids();
    assert_ne!(top[0], bottom[0]);

    let top = store.get(node, top[0]).unwrap();
    let bottom = store.get(node, bottom[0]).unwrap();
    assert_ne!(top, bottom);
    assert_eq!(store.len(), 2);
    assert_eq!(world.get(top).text, "up");
    assert_eq!(world.get(bottom).text, "down");
}

#[test]
fn patching_one_of_a_pair_leaves_the_other() {
    let (mut world, parent) = World::with_root();
    let mut store = Store::new();
    let mut pair = a_pair();
    let node = pair.build(&mut world, parent, &mut store);
    let top = store.get(node, Pair::path().top().ids()[0]).unwrap();
    let bottom =
        store.get(node, Pair::path().bottom().ids()[0]).unwrap();

    let path = Pair::path().bottom().text();
    *(path.accessor().get_mut)(&mut pair).unwrap() = "DOWN".into();
    pair.patch(&mut world, node, &path.ids(), &mut store);

    assert_eq!(world.get(bottom).text, "DOWN");
    assert_eq!(world.get(top).text, "up");
}
