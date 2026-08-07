//! A backend, and the one element every test needs.
//!
//! Nodes are keys into a map, so despawning one leaves every other
//! handle pointing where it did. Stands in for an ECS or a retained
//! scene graph.

// Each test file uses a different part of this.
#![allow(dead_code)]

use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::host::Host;
use fynix_mock::lenz::Lenz;
use hashbrown::HashMap;

/// Whatever the elements under test write. A real backend would have
/// components; this one has a field per thing anybody writes.
#[derive(Debug, Default, PartialEq)]
pub struct Node {
    parent: Option<usize>,
    pub text: String,
    pub glyph: char,
    pub size: u32,
    pub padding: u32,
    pub border_width: u32,
    pub border_radius: u32,
}

#[derive(Default)]
pub struct World {
    nodes: HashMap<usize, Node>,
    /// Never reused, so a handle to a despawned node cannot come back
    /// as some later one.
    next: usize,
}

impl World {
    /// A world holding nothing but a root, and that root.
    pub fn with_root() -> (Self, usize) {
        let mut world = Self::default();
        let root = world.insert(Node::default());
        (world, root)
    }

    fn insert(&mut self, node: Node) -> usize {
        let id = self.next;
        self.next += 1;
        self.nodes.insert(id, node);
        id
    }

    pub fn node(&mut self, node: usize) -> &mut Node {
        self.nodes.get_mut(&node).expect("live node")
    }

    pub fn get(&self, node: usize) -> &Node {
        self.nodes.get(&node).expect("live node")
    }
}

pub struct Backend;

impl Host for Backend {
    type Node = usize;
    type World = World;

    fn spawn(world: &mut World, parent: usize) -> usize {
        world.insert(Node {
            parent: Some(parent),
            ..Node::default()
        })
    }

    fn exists(world: &World, node: usize) -> bool {
        world.nodes.contains_key(&node)
    }

    fn children(world: &World, node: usize) -> Vec<usize> {
        let mut children: Vec<usize> = world
            .nodes
            .iter()
            .filter(|(_, child)| child.parent == Some(node))
            .map(|(id, _)| *id)
            .collect();

        // A map has no order of its own, and the trait promises one.
        // Keys count up, so sorting them is spawn order.
        children.sort_unstable();
        children
    }

    fn despawn(world: &mut World, node: usize) {
        for child in Self::children(world, node) {
            Self::despawn(world, child);
        }
        world.nodes.remove(&node);
    }
}

#[derive(Lenz, Element)]
pub struct Label {
    pub text: String,
    pub size: u32,
}

impl ElementVisual<Backend> for Label {
    fn build_fields(&self, world: &mut World, node: usize) {
        world.node(node).text = self.text.clone();
        world.node(node).size = self.size;
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: usize,
        field: LabelField,
    ) {
        match field {
            LabelField::Text => {
                world.node(node).text = self.text.clone()
            }
            LabelField::Size => world.node(node).size = self.size,
        }
    }
}

/// A label, for tests that only care that a child exists.
pub fn a_label() -> Label {
    Label {
        text: "Save".into(),
        size: 13,
    }
}
