//! Bevy backend for [`fynix_mock`].
//!
//! The seam, and nothing else: nodes are entities, the world is
//! [`World`], and the kernel is a resource flushed once a frame.
//! Elements and styles live above this.

pub mod host;
pub mod interact;

use core::marker::PhantomData;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::system::IntoObserverSystem;
use fynix_mock::Fynix;
use fynix_mock::element::Element;
use fynix_mock::records::BuildFn;
use fynix_mock::ui::{Build, ElementMut, Ui};

use crate::host::BevyHost;

/// Runs [`Fynix::flush`] in [`FynixSet`], every [`Update`]. `Theme`
/// is whatever the app's own [`Resource`] is - this crate never
/// names it, only that one exists.
pub struct FynixPlugin<Theme>(PhantomData<fn() -> Theme>);

impl<Theme> Default for FynixPlugin<Theme> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<Theme: Resource + Clone + Default> Plugin
    for FynixPlugin<Theme>
{
    fn build(&self, app: &mut App) {
        // `Theme` is a `Resource` this crate takes on faith, not one
        // it defines - initializing it here (it's already bound
        // `Default`) is what lets an app that has no opinion on its
        // own theme, and every test, add the plugin without setting
        // one up first. The kernel takes its own copy at construction
        // rather than reading `Theme` back out of `World` each flush
        // - see [`fynix_mock::Host::Theme`] - so it's seeded here,
        // once, from whatever `Theme` now holds.
        app.init_resource::<Theme>();
        let theme = app.world().resource::<Theme>().clone();

        app.insert_resource(BevyFynix(Fynix::new(theme)))
            .add_systems(
                Update,
                (sync_theme::<Theme>, flush::<Theme>)
                    .chain()
                    .in_set(FynixSet),
            );
    }
}

/// What a build takes.
pub type BevyUi<'a, Theme> = Ui<'a, BevyHost<Theme>>;

/// Private, because a flush owns the kernel for as long as it runs and
/// anything it builds could otherwise borrow it again. Watchers are
/// declared inside a build, and the first one through [`watch_root`].
#[derive(Resource)]
struct BevyFynix<Theme: Resource + Clone + Default>(
    Fynix<BevyHost<Theme>>,
);

/// Order systems against the flush: whatever a build reads should be
/// written before [`FynixSet`] runs.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FynixSet;

/// Pushes an edited `Theme` resource into the kernel, which is what
/// actually schedules the full rebuild - see
/// [`Fynix::theme_mut`](fynix_mock::Fynix::theme_mut). Ordered ahead
/// of [`flush`] so the same frame's rebuild sees the new theme rather
/// than lagging a frame behind it.
fn sync_theme<Theme: Resource + Clone + Default>(
    theme: Res<Theme>,
    mut kernel: ResMut<BevyFynix<Theme>>,
) {
    if theme.is_changed() {
        *kernel.0.theme_mut() = theme.clone();
    }
}

/// Build `root` on the next flush, and never again.
///
/// The bootstrap: everything reactive below it is declared inside
/// `build`. Call it once per root, after spawning that root.
pub fn watch_root<Theme: Resource + Clone + Default>(
    world: &mut World,
    root: Entity,
    build: impl BuildFn<BevyHost<Theme>>,
) {
    let mut pending = true;

    world.resource_mut::<BevyFynix<Theme>>().0.watch(
        root,
        move |_, _| core::mem::take(&mut pending),
        build,
    );
}

fn flush<Theme: Resource + Clone + Default>(world: &mut World) {
    with_kernel::<Theme>(world, |kernel, world| kernel.flush(world));
}

/// Run `f` with the kernel out of the world, which is the only way to
/// have both. Not for anything a flush can reach: the kernel is gone
/// from the world for as long as this runs.
pub(crate) fn with_kernel<Theme: Resource + Clone + Default>(
    world: &mut World,
    f: impl FnOnce(&mut Fynix<BevyHost<Theme>>, &mut World),
) {
    world.resource_scope(
        |world, mut kernel: Mut<BevyFynix<Theme>>| {
            f(&mut kernel.0, world);
        },
    );
}

/// What bevy wants on a node that the element itself has no say in.
///
/// `observe`/`insert`/`remove` are all just `entity_mut()` plus
/// whatever `bevy_ecs` already offers - the one real difference
/// between `ElementMut` and `Build` is how each reaches the world in
/// the first place, so that is the only method either has to provide.
pub trait EntityExt {
    /// This node itself, for whatever `bevy_ecs` offers that has no
    /// shorthand of its own here - `.entity_mut().insert(...)` rather
    /// than reaching for the world and the node by hand.
    fn entity_mut(&mut self) -> EntityWorldMut<'_>;

    /// Watch this node for `V`.
    fn observe<V: EntityEvent, B: Bundle, M>(
        &mut self,
        observer: impl IntoObserverSystem<V, B, M>,
    ) -> &mut Self {
        self.entity_mut().observe(observer);
        self
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
}

impl<E, T> EntityExt for ElementMut<'_, '_, BevyHost<T>, E>
where
    T: Resource + Clone + Default,
    E: Element<BevyHost<T>>,
{
    fn entity_mut(&mut self) -> EntityWorldMut<'_> {
        let node = self.id();
        self.ui.world.entity_mut(node)
    }
}

impl<E, T> EntityExt for Build<'_, BevyHost<T>, E>
where
    E: Element<BevyHost<T>>,
    T: Resource + Clone + Default,
{
    fn entity_mut(&mut self) -> EntityWorldMut<'_> {
        let node = self.id();
        self.world.entity_mut(node)
    }
}
