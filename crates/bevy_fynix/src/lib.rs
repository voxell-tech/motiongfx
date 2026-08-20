//! Bevy backend for [`fynix_mock`].
//!
//! The seam, and nothing else: nodes are entities, the world is
//! [`World`], and the kernel is a resource flushed once a frame.
//! Elements and styles live above this.

pub mod host;
pub mod interact;

use core::ops::{Deref, DerefMut};
use std::sync::Mutex;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::system::IntoObserverSystem;
use fynix_mock::Fynix;
use fynix_mock::element::Element;
use fynix_mock::records::BuildFn;
use fynix_mock::ui::{Build, ElementMut, Patch, Ui};

use crate::host::BevyHost;

/// Runs [`Fynix::flush`] in [`FynixSet`], every [`Update`]. `Theme` is
/// the app's own type - never a [`Resource`], never read back out of
/// `World`; see [`theme`] and [`theme_mut`] for reaching it from
/// outside a build.
pub struct FynixPlugin<Theme>(Mutex<Option<Theme>>);

impl<Theme> FynixPlugin<Theme> {
    /// Starts the kernel themed with `theme`.
    pub fn new(theme: Theme) -> Self {
        Self(Mutex::new(Some(theme)))
    }
}

impl<Theme: Default> Default for FynixPlugin<Theme> {
    fn default() -> Self {
        Self::new(Theme::default())
    }
}

impl<Theme: Send + Sync + 'static> Plugin for FynixPlugin<Theme> {
    fn build(&self, app: &mut App) {
        let theme = self
            .0
            .lock()
            .unwrap()
            .take()
            .expect("a plugin's own build runs once");

        app.insert_resource(BevyFynix(Fynix::new(theme)))
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

/// Build `root` on the next flush, and never again.
///
/// Everything reactive below it is declared inside `build`. Call it
/// once per root, after spawning that root.
pub fn watch_root<Theme: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    build: impl BuildFn<BevyHost<Theme>>,
) {
    let mut pending = true;

    world.resource_mut::<BevyFynix<Theme>>().watch(
        root,
        move |_, _| core::mem::take(&mut pending),
        build,
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

/// Entity-level operations on whichever node `Build`, `Patch`, or
/// `ElementMut` currently holds, built on [`Self::id`] and
/// [`Self::world_mut`] plus whatever `bevy_ecs` offers on top.
pub trait EntityExt {
    /// This node's own handle.
    fn id(&self) -> Entity;

    /// The world this node lives in.
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
}

impl<E, T> EntityExt for ElementMut<'_, '_, BevyHost<T>, E>
where
    T: Send + Sync + 'static,
    E: Element<BevyHost<T>>,
{
    fn id(&self) -> Entity {
        ElementMut::id(self)
    }

    fn world_mut(&mut self) -> &mut World {
        self.ui.world
    }
}

impl<E, T> EntityExt for Build<'_, BevyHost<T>, E>
where
    E: Element<BevyHost<T>>,
    T: Send + Sync + 'static,
{
    fn id(&self) -> Entity {
        Build::id(self)
    }

    fn world_mut(&mut self) -> &mut World {
        self.world
    }
}

impl<T> EntityExt for Patch<'_, BevyHost<T>>
where
    T: Send + Sync + 'static,
{
    fn id(&self) -> Entity {
        Patch::id(self)
    }

    fn world_mut(&mut self) -> &mut World {
        self.world
    }
}
