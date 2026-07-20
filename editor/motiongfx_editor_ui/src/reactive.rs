//! Bevy backend for [`motiongfx_editor_ui_kernel`].
//!
//! The kernel is bevy-free; this is where it meets the ECS. Nodes are
//! entities, widgets are boxed [`Scene`]s (so `bsn!` still builds each
//! individual widget), and the world is [`World`].

use std::sync::Arc;

use bevy::ecs::change_detection::{ComponentTicks, Tick};
use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use bevy::scene::EntityWorldMutSceneExt;
use bevy::ui::Node as UiNode;

use motiongfx_editor_ui_kernel::{
    ChangedFn, Host, Kernel, NodeMut, Ui,
};

/// The kernel itself. Private: the app never touches it directly,
/// because a flush owns it exclusively and anything the flush spawns
/// could otherwise try to borrow it re-entrantly. Register through
/// [`Ui`] instead: every watcher and binding is declared inside a
/// build, and the one bootstrap watcher is registered here.
#[derive(Resource, Deref, DerefMut, Default)]
struct BevyKernel(Kernel<BevyHost>);

/// Marks the entity the whole UI tree is built under. Spawn exactly
/// one, carrying whatever the app's root node needs (camera target,
/// root `Node`); the kernel fills it from the builder given to
/// [`KernelPlugin::new`].
#[derive(Component, Default, Clone)]
pub struct KernelRoot;

type RootBuildFn = Arc<dyn for<'a> Fn(&mut BevyUi<'a>) + Send + Sync>;

#[derive(Resource, Deref)]
struct RootBuild(RootBuildFn);

/// Drives [`Kernel::flush`] once per frame and owns the bootstrap:
/// the one watcher the app cannot declare through [`Ui`], because
/// every other watcher and binding is nested inside a build.
pub struct KernelPlugin(RootBuildFn);

impl KernelPlugin {
    pub fn new(
        build: impl for<'a> Fn(&mut BevyUi<'a>) + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(build))
    }
}

impl Plugin for KernelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BevyKernel>()
            .insert_resource(RootBuild(self.0.clone()))
            .add_systems(
                Update,
                (watch_root, flush_kernel).chain().in_set(KernelSet),
            );
    }
}

/// The root build runs once; everything reactive below it is a nested
/// [`Ui::watch`].
fn watch_root(
    mut kernel: ResMut<BevyKernel>,
    root_build: Res<RootBuild>,
    q_root: Query<Entity, Added<KernelRoot>>,
) {
    for root in &q_root {
        let build = root_build.0.clone();
        let mut first = true;
        kernel.watch(
            root,
            move |_, _| std::mem::replace(&mut first, false),
            move |ui| build(ui),
        );
    }
}

fn flush_kernel(world: &mut World) {
    world.resource_scope(|world, mut kernel: Mut<BevyKernel>| {
        kernel.flush(world);
    });
}

/// Order app systems against the flush: anything the builders read
/// should be up to date before [`KernelSet`] runs.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct KernelSet;

/// Builders take this: `fn build(ui: &mut BevyUi)`.
pub type BevyUi<'a> = Ui<'a, BevyHost>;

/// The bevy [`Host`].
pub struct BevyHost;

impl Host for BevyHost {
    type Node = Entity;
    type World = World;

    fn spawn(
        world: &mut Self::World,
        parent: Self::Node,
    ) -> Self::Node {
        // A layout `bevy_ui::Node` up front (not `Self::Node`, which
        // is the `Entity` handle): an entity in the UI tree without
        // one is always a mistake, and nodes spawned without a widget
        // (`watch`, `group`, `bind_raw`) would otherwise have none.
        // Bevy warns B0004 and skips their children's layout. A widget
        // that brings its own overwrites this.
        world.spawn((UiNode::default(), ChildOf(parent))).id()
    }

    fn exists(world: &Self::World, node: Self::Node) -> bool {
        world.entities().contains(node)
    }

    fn children(
        world: &Self::World,
        node: Self::Node,
    ) -> Vec<Self::Node> {
        world
            .get::<Children>(node)
            .map(|children| children.to_vec())
            .unwrap_or_default()
    }

    fn despawn(world: &mut Self::World, node: Self::Node) {
        world.despawn(node);
    }
}

/// Typed bindings, kept out of the kernel: only here are the component
/// types still in scope.
pub trait BevyNodeMutExt {
    /// Write `C` on this node whenever `changed` fires.
    fn bind<C: Component>(
        self,
        changed: impl ChangedFn<BevyHost>,
        value: impl Fn(&World, Entity) -> C + Send + Sync + 'static,
    ) -> Self;

    /// Like [`Self::bind`], but writes one field instead of replacing
    /// the component. The node-local counterpart of
    /// [`BevyUiExt::bind_field`]; same `get`/`set` reasoning.
    fn bind_field<C: Component<Mutability = Mutable>, T>(
        self,
        changed: impl ChangedFn<BevyHost>,
        get: impl Fn(&World, Entity) -> T + Send + Sync + 'static,
        set: impl Fn(&mut C, T) + Send + Sync + 'static,
    ) -> Self
    where
        T: Send + Sync + 'static;
}

impl BevyNodeMutExt for NodeMut<'_, '_, BevyHost> {
    fn bind<C: Component>(
        self,
        changed: impl ChangedFn<BevyHost>,
        value: impl Fn(&World, Entity) -> C + Send + Sync + 'static,
    ) -> Self {
        self.bind_raw(changed, move |world, node| {
            let component = value(world, node);
            world.entity_mut(node).insert(component);
        })
    }

    fn bind_field<C: Component<Mutability = Mutable>, T>(
        self,
        changed: impl ChangedFn<BevyHost>,
        get: impl Fn(&World, Entity) -> T + Send + Sync + 'static,
        set: impl Fn(&mut C, T) + Send + Sync + 'static,
    ) -> Self
    where
        T: Send + Sync + 'static,
    {
        self.bind_raw(changed, move |world, node| {
            let value = get(world, node);
            let mut entity = world.entity_mut(node);
            let Some(mut component) = entity.get_mut::<C>() else {
                return;
            };
            set(&mut component, value);
        })
    }
}

/// Spawning a node from a scene. Reactivity is declared on the node
/// afterwards, through [`BevyNodeMutExt`].
pub trait BevyUiExt<'a> {
    /// Add a node built from a `bsn!` scene.
    fn bsn(&mut self, scene: impl Scene)
    -> NodeMut<'_, 'a, BevyHost>;
}

impl<'a> BevyUiExt<'a> for BevyUi<'a> {
    fn bsn(
        &mut self,
        scene: impl Scene,
    ) -> NodeMut<'_, 'a, BevyHost> {
        self.node(move |world: &mut World, node: Entity| {
            if let Err(err) =
                world.entity_mut(node).apply_scene(scene)
            {
                error!("failed to build node {node}: {err}");
            }
        })
    }
}

/// Fires when `R` changed since the last poll. Also fires on the first
/// poll, so a binding starts out in sync with the world.
pub fn resource_changed<R: Resource>()
-> impl FnMut(&World, Entity) -> bool {
    let mut seen: Option<Tick> = None;
    move |world, _| {
        let Some(ticks) = world.get_resource_change_ticks::<R>()
        else {
            return false;
        };
        let ComponentTicks { changed, .. } = ticks;
        let fired = seen != Some(changed);
        seen = Some(changed);
        fired
    }
}

/// Fires when a *projection* of `R` changes, ignoring every other
/// mutation.
///
/// This is what lets `watch` mean "structure" and `bind` mean "value".
/// Watching a whole resource rebuilds on any field change: dragging a
/// splitter nudges `DockTree`'s fractions every frame, which would
/// rebuild the layout every frame. Project to the structural part and
/// the drag fires nothing; a `bind` carries the ratio instead.
pub fn structure_changed<R: Resource, K>(
    project: impl Fn(&R) -> K + Send + Sync + 'static,
) -> impl FnMut(&World, Entity) -> bool
where
    K: PartialEq + Send + Sync + 'static,
{
    let mut seen: Option<K> = None;
    move |world, _| {
        let Some(resource) = world.get_resource::<R>() else {
            return false;
        };
        let current = project(resource);
        let fired = seen.as_ref() != Some(&current);
        seen = Some(current);
        fired
    }
}

/// Fires when `read`'s value differs from the last poll.
///
/// For signals that are entity state rather than a resource, where
/// there is no tick to compare. `read` runs every flush, so it must be
/// cheap: resolve entities *outside* the closure (a registering system
/// has queries; a predicate only has `&World`, where finding an entity
/// by component means scanning the whole world).
pub fn value_changed<T>(
    read: impl Fn(&World, Entity) -> T + Send + Sync + 'static,
) -> impl FnMut(&World, Entity) -> bool + Send + Sync + 'static
where
    T: PartialEq + Send + Sync + 'static,
{
    let mut seen: Option<T> = None;
    move |world, node| {
        let current = read(world, node);
        let fired = seen.as_ref() != Some(&current);
        seen = Some(current);
        fired
    }
}

/// Fires when the current `S` differs from the last poll.
pub fn state_changed<S: States>() -> impl FnMut(&World, Entity) -> bool
{
    let mut seen: Option<S> = None;
    move |world, _| {
        let current =
            world.get_resource::<State<S>>().map(|s| s.get().clone());
        let fired = seen != current;
        seen = current;
        fired
    }
}
