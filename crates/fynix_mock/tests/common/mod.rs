//! A backend, and the one element every test needs.
//!
//! Nodes are keys into a map, so despawning one leaves every other
//! handle pointing where it did. Stands in for an ECS or a retained
//! scene graph.

// Each test file uses a different part of this.
#![allow(dead_code)]

use fynix_mock::Fynix;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::host::Host;
use fynix_mock::lenz::{Cursor, FieldPath, Identity};
use fynix_mock::ui::{Build, ElementMut, Patch};
use hashbrown::HashMap;

/// What this test stands in for a pointer with: not fynix's concern,
/// so it is defined here rather than imported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Interact {
    Enter,
    Leave,
}

/// What a backend does when an interaction fires: point a lane
/// somewhere. Only needs the kernel, since [`Fynix::aim`] does too.
type Aim = Box<dyn Fn(&mut Fynix<Backend>) + Send + Sync>;

/// The `aim_on` a real backend would build for itself, over whatever
/// events it actually has. This one is a stand-in for a pointer.
pub trait TestAim<E> {
    fn aim_on<P>(
        &mut self,
        on: Interact,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        target: Option<P::Target>,
    ) -> &mut Self
    where
        P: FieldPath<Source = E>,
        P::Target: Clone + Send + Sync;
}

impl<E: Element<Backend>> TestAim<E>
    for ElementMut<'_, '_, Backend, E>
{
    fn aim_on<P>(
        &mut self,
        on: Interact,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        target: Option<P::Target>,
    ) -> &mut Self
    where
        P: FieldPath<Source = E>,
        P::Target: Clone + Send + Sync,
    {
        let node = self.id();
        self.ui.world.interactions.push((
            node,
            on,
            Box::new(move |kernel: &mut Fynix<Backend>| {
                kernel.aim(node, field, target.clone());
            }),
        ));
        self
    }
}

/// As above, from [`build_fields`](ElementVisual::build_fields):
/// [`Build`] carries `world` directly rather than through a [`Ui`], but
/// otherwise wires the same interaction the same way.
impl<E: Element<Backend>> TestAim<E> for Build<'_, Backend, E> {
    fn aim_on<P>(
        &mut self,
        on: Interact,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        target: Option<P::Target>,
    ) -> &mut Self
    where
        P: FieldPath<Source = E>,
        P::Target: Clone + Send + Sync,
    {
        let node = self.id();
        self.world.interactions.push((
            node,
            on,
            Box::new(move |kernel: &mut Fynix<Backend>| {
                kernel.aim(node, field, target.clone());
            }),
        ));
        self
    }
}

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

/// What a predicate watches and a binding reads. A real app would
/// have state of its own; these tests need somewhere to put a change.
#[derive(Default)]
pub struct Source {
    pub text: String,
    pub changed: bool,
}

#[derive(Default)]
pub struct World {
    nodes: HashMap<usize, Node>,
    /// Never reused, so a handle to a despawned node cannot come back
    /// as some later one.
    next: usize,
    pub source: Source,
    /// What a flush advances a transition by. A test sets it outright
    /// rather than owning a clock.
    pub delta: f32,
    /// What a style asked to be told about. A real backend would hand
    /// these to its pointer; a test fires them by hand.
    interactions: Vec<(usize, Interact, Aim)>,
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

    /// Do to `node` what a pointer would, and run whatever asked to
    /// hear about it.
    pub fn interact(
        &mut self,
        kernel: &mut Fynix<Backend>,
        node: usize,
        on: Interact,
    ) {
        // Taken out for the call: an aim is handed the world it was
        // registered in.
        let interactions = core::mem::take(&mut self.interactions);

        for (target, kind, aim) in &interactions {
            if (*target, *kind) == (node, on) {
                aim(kernel);
            }
        }

        self.interactions = interactions;
    }
}

pub struct Backend;

impl Host for Backend {
    type Node = usize;
    type World = World;
    /// Nothing in these tests reads a theme.
    type Theme = ();

    fn delta(world: &World) -> f32 {
        world.delta
    }

    fn theme(_world: &World) {}

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

/// The default is what a test gets when it only cares that a label is
/// there, so nothing has to spell one out.
#[derive(Element)]
pub struct Label {
    #[default(String::from("Label"))]
    pub text: String,
    #[default(13)]
    pub size: u32,
}

impl ElementVisual<Backend> for Label {
    fn build_fields(&self, draw: &mut Build<Backend, Self>) {
        let node = draw.id();
        let world = &mut *draw.world;

        world.node(node).text = self.text.clone();
        world.node(node).size = self.size;
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<Backend>,
        field: LabelField,
    ) {
        let node = patch.id();
        let world = &mut *patch.world;

        match field {
            LabelField::Text => {
                world.node(node).text = self.text.clone()
            }
            LabelField::Size => world.node(node).size = self.size,
        }
    }
}
