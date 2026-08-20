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

/// Runs [`Fynix::flush`] in [`FynixSet`], every [`Update`]. `Theme` is
/// the app's own [`Resource`]; this crate never names it.
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
        // Seeds the kernel with whatever `Theme` holds now. The
        // kernel does not read `Theme` back out of `World` later.
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

/// A flush owns the kernel for as long as it runs, and anything it
/// builds could otherwise borrow it again.
#[derive(Resource)]
struct BevyFynix<Theme: Resource + Clone + Default>(
    Fynix<BevyHost<Theme>>,
);

/// Order systems against the flush: whatever a build reads should be
/// written before [`FynixSet`] runs.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FynixSet;

/// Pushes an edited `Theme` resource into the kernel. Ordered ahead
/// of [`flush`], so the same frame's rebuild sees the new theme.
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
/// Everything reactive below it is declared inside `build`. Call it
/// once per root, after spawning that root.
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

/// Run `f` with the kernel taken out of the world. Not for anything
/// a flush can reach.
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
/// `observe`/`insert`/`remove` are just `entity_mut()` plus whatever
/// `bevy_ecs` offers. Only `entity_mut` differs between `ElementMut`
/// and `Build`.
pub trait EntityExt {
    /// This node itself, for whatever `bevy_ecs` offers with no
    /// shorthand here.
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
