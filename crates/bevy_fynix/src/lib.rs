//! Bevy backend for [`fynix_mock`].
//!
//! The seam, and nothing else: nodes are entities, the world is
//! [`World`], and the kernel is a resource flushed once a frame.
//! Elements and styles live above this.

pub mod host;
pub mod interact;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::system::IntoObserverSystem;
use fynix_mock::Fynix;
use fynix_mock::element::Element;
use fynix_mock::ui::{BuildFn, ElementMut, Ui};

use crate::host::BevyHost;

/// Runs [`Fynix::flush`] in [`FynixSet`], every [`Update`].
pub struct FynixPlugin;

impl Plugin for FynixPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BevyFynix>()
            .add_systems(Update, flush.in_set(FynixSet));
    }
}

/// What a build takes.
pub type BevyUi<'a> = Ui<'a, BevyHost>;

/// Private, because a flush owns the kernel for as long as it runs and
/// anything it builds could otherwise borrow it again. Watchers are
/// declared inside a build, and the first one through [`watch_root`].
#[derive(Resource, Default)]
struct BevyFynix(Fynix<BevyHost>);

/// Order systems against the flush: whatever a build reads should be
/// written before [`FynixSet`] runs.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FynixSet;

/// Build `root` on the next flush, and never again.
///
/// The bootstrap: everything reactive below it is declared inside
/// `build`. Call it once per root, after spawning that root.
pub fn watch_root(
    world: &mut World,
    root: Entity,
    build: impl BuildFn<BevyHost>,
) {
    let mut pending = true;

    world.resource_mut::<BevyFynix>().0.watch(
        root,
        move |_, _| core::mem::take(&mut pending),
        build,
    );
}

fn flush(world: &mut World) {
    with_kernel(world, |kernel, world| kernel.flush(world));
}

/// Run `f` with the kernel out of the world, which is the only way to
/// have both. Not for anything a flush can reach: the kernel is gone
/// from the world for as long as this runs.
pub(crate) fn with_kernel(
    world: &mut World,
    f: impl FnOnce(&mut Fynix<BevyHost>, &mut World),
) {
    world.resource_scope(|world, mut kernel: Mut<BevyFynix>| {
        f(&mut kernel.0, world);
    });
}

/// What bevy wants on a node that the element itself has no say in.
pub trait ElementMutExt<E: Element<BevyHost>> {
    /// Watch this node for `V`.
    fn observe<V: EntityEvent, B: Bundle, M>(
        self,
        observer: impl IntoObserverSystem<V, B, M>,
    ) -> Self;

    /// Put `bundle` on this node, once, now.
    fn insert(self, bundle: impl Bundle) -> Self;
}

impl<E: Element<BevyHost>> ElementMutExt<E>
    for ElementMut<'_, '_, BevyHost, E>
{
    fn observe<V: EntityEvent, B: Bundle, M>(
        self,
        observer: impl IntoObserverSystem<V, B, M>,
    ) -> Self {
        let node = self.id();
        self.ui.world.entity_mut(node).observe(observer);
        self
    }

    fn insert(self, bundle: impl Bundle) -> Self {
        let node = self.id();
        self.ui.world.entity_mut(node).insert(bundle);
        self
    }
}

