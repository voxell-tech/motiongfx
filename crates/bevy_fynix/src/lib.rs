//! Bevy backend for [`fynix`].
//!
//! The seam, and nothing else: nodes are entities, the world is
//! [`World`], and the kernel is a resource flushed once a frame.
//! Elements and styles live above this.

pub mod host;
pub mod interact;
pub mod tag;

use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use bevy_app::prelude::*;
use bevy_ecs::component::Mutable;
use bevy_ecs::prelude::*;
use bevy_ecs::system::IntoObserverSystem;
use fynix::Fynix;
use fynix::element::Element;
use fynix::records::BuildFn;
use fynix::ui::{Build, ElementMut, Patch, Ui};
use fynix::world_node::{WorldNodeMut, WorldNodeRef};

use crate::host::BevyHost;

/// Runs [`Fynix::flush`] in [`FynixSet`], every [`Update`]. `Theme` is
/// the app's own type - never a [`Resource`], never read back out of
/// `World`. Starts the kernel with `Theme::default()`; for anything
/// else, edit it after the fact through [`theme_mut`].
pub struct FynixPlugin<Theme>(PhantomData<fn() -> Theme>);

impl<Theme> Default for FynixPlugin<Theme> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<Theme: Default + Send + Sync + 'static> Plugin
    for FynixPlugin<Theme>
{
    fn build(&self, app: &mut App) {
        app.insert_resource(BevyFynix(Fynix::new(Theme::default())))
            .add_systems(Update, flush::<Theme>.in_set(FynixSet));
    }
}

/// What a build takes.
pub type BevyUi<'a, Theme> = Ui<'a, BevyHost<Theme>>;

/// A flush owns the kernel for as long as it runs, and anything it
/// builds could otherwise borrow it again. Transparent otherwise -
/// [`Deref`]/[`DerefMut`] reach straight through to the [`Fynix`]
/// underneath.
#[derive(Resource)]
pub struct BevyFynix<Theme: Send + Sync + 'static>(
    Fynix<BevyHost<Theme>>,
);

impl<Theme: Send + Sync + 'static> Deref for BevyFynix<Theme> {
    type Target = Fynix<BevyHost<Theme>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Theme: Send + Sync + 'static> DerefMut for BevyFynix<Theme> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Order systems against the flush: whatever a build reads should be
/// written before [`FynixSet`] runs.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FynixSet;

/// The theme the kernel is currently building with.
///
/// Borrowed straight out of the kernel's own resource - `Theme` lives
/// nowhere else in `World`.
pub fn theme<Theme: Send + Sync + 'static>(world: &World) -> &Theme {
    world.resource::<BevyFynix<Theme>>().theme()
}

/// The theme, to edit in place. Schedules a full rebuild for the
/// next flush - see [`Fynix::theme_mut`].
pub fn theme_mut<Theme: Send + Sync + 'static>(
    world: &mut World,
) -> &mut Theme {
    let kernel = world.resource_mut::<BevyFynix<Theme>>();
    kernel.into_inner().theme_mut()
}

/// Build `root` immediately, and never again.
///
/// Everything reactive below it is declared inside `build`. Call it
/// once per root, after spawning that root.
pub fn watch_root<Theme: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    build: impl BuildFn<BevyHost<Theme>>,
) {
    let mut pending = true;

    world.resource_scope::<BevyFynix<Theme>, _>(
        |world, mut kernel| {
            kernel.watch(
                root,
                move |_| core::mem::take(&mut pending),
                build,
                world,
            );
        },
    );
}

fn flush<Theme: Send + Sync + 'static>(world: &mut World) {
    with_kernel::<Theme>(world, |kernel, world| kernel.flush(world));
}

/// Run `f` with the kernel taken out of the world. Not for anything
/// a flush can reach.
pub(crate) fn with_kernel<Theme: Send + Sync + 'static>(
    world: &mut World,
    f: impl FnOnce(&mut Fynix<BevyHost<Theme>>, &mut World),
) {
    world.resource_scope(
        |world, mut kernel: Mut<BevyFynix<Theme>>| {
            f(&mut kernel, world);
        },
    );
}

/// Read-only node context: the node's handle and the world it lives
/// in, plus shorthands built on the two. Implemented by everything
/// that carries a `(world, node)` pair, `WorldNodeRef` included.
pub trait WorldEntityRef {
    /// This node's own handle.
    fn id(&self) -> Entity;

    /// The world this node lives in.
    fn world(&self) -> &World;

    /// The world's `R`. Panics if it is absent.
    fn resource<R: Resource>(&self) -> &R {
        self.world().resource::<R>()
    }
}

/// Entity-level operations on whichever node `Build`, `Patch`, or
/// `ElementMut` currently holds, built on [`WorldEntityRef::id`] and
/// [`Self::world_mut`] plus whatever `bevy_ecs` offers on top.
pub trait WorldEntityMut: WorldEntityRef {
    /// The world this node lives in, mutably.
    fn world_mut(&mut self) -> &mut World;

    /// This node itself, for whatever `bevy_ecs` offers with no
    /// shorthand here.
    fn entity_mut(&mut self) -> EntityWorldMut<'_> {
        let node = self.id();
        self.world_mut().entity_mut(node)
    }

    /// Put `bundle` on this node, once, now.
    fn insert(&mut self, bundle: impl Bundle) -> &mut Self {
        self.entity_mut().insert(bundle);
        self
    }

    /// Take `B` off this node, once, now.
    fn remove<B: Bundle>(&mut self) -> &mut Self {
        self.entity_mut().remove::<B>();
        self
    }

    /// Spawn `bundle` as a new child of this node, once, now.
    fn with_child(&mut self, bundle: impl Bundle) -> &mut Self {
        self.entity_mut().with_child(bundle);
        self
    }

    /// Spawn each child `func` adds, on this node, once, now.
    fn with_children(
        &mut self,
        func: impl FnOnce(&mut ChildSpawner),
    ) -> &mut Self {
        self.entity_mut().with_children(func);
        self
    }

    /// Watch this node for `V`.
    fn observe<V: EntityEvent, B: Bundle, M>(
        &mut self,
        observer: impl IntoObserverSystem<V, B, M>,
    ) -> &mut Self {
        self.entity_mut().observe(observer);
        self
    }

    /// The world's `R`, mutably. Panics if it is absent.
    fn resource_mut<R: Resource<Mutability = Mutable>>(
        &mut self,
    ) -> Mut<'_, R> {
        self.world_mut().resource_mut::<R>()
    }
}

impl<E, T> WorldEntityRef for ElementMut<'_, '_, BevyHost<T>, E>
where
    T: Send + Sync + 'static,
    E: Element<BevyHost<T>>,
{
    fn id(&self) -> Entity {
        ElementMut::id(self)
    }

    fn world(&self) -> &World {
        self.ui.world
    }
}

impl<E, T> WorldEntityMut for ElementMut<'_, '_, BevyHost<T>, E>
where
    T: Send + Sync + 'static,
    E: Element<BevyHost<T>>,
{
    fn world_mut(&mut self) -> &mut World {
        self.ui.world
    }
}

impl<E, T> WorldEntityRef for Build<'_, BevyHost<T>, E>
where
    E: Element<BevyHost<T>>,
    T: Send + Sync + 'static,
{
    fn id(&self) -> Entity {
        Build::id(self)
    }

    fn world(&self) -> &World {
        self.world
    }
}

impl<E, T> WorldEntityMut for Build<'_, BevyHost<T>, E>
where
    E: Element<BevyHost<T>>,
    T: Send + Sync + 'static,
{
    fn world_mut(&mut self) -> &mut World {
        self.world
    }
}

impl<T> WorldEntityRef for Patch<'_, BevyHost<T>>
where
    T: Send + Sync + 'static,
{
    fn id(&self) -> Entity {
        Patch::id(self)
    }

    fn world(&self) -> &World {
        self.world
    }
}

impl<T> WorldEntityMut for Patch<'_, BevyHost<T>>
where
    T: Send + Sync + 'static,
{
    fn world_mut(&mut self) -> &mut World {
        self.world
    }
}

/// So a predicate or value reader handed a [`WorldNodeRef`] can reach
/// a resource with `world_node.resource::<R>()`.
impl<T> WorldEntityRef for WorldNodeRef<'_, BevyHost<T>>
where
    T: Send + Sync + 'static,
{
    fn id(&self) -> Entity {
        self.node
    }

    fn world(&self) -> &World {
        self.world
    }
}

/// So a free function whose whole job is to mutate its own node (or
/// reach one child) can take a [`WorldNodeMut`] and use the same
/// entity shorthands `Build` and `Patch` offer, rather than a manual
/// `world.entity_mut(node)`.
impl<T> WorldEntityRef for WorldNodeMut<'_, BevyHost<T>>
where
    T: Send + Sync + 'static,
{
    fn id(&self) -> Entity {
        self.node
    }

    fn world(&self) -> &World {
        self.world
    }
}

impl<T> WorldEntityMut for WorldNodeMut<'_, BevyHost<T>>
where
    T: Send + Sync + 'static,
{
    fn world_mut(&mut self) -> &mut World {
        self.world
    }
}
