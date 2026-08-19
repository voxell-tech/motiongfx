//! The backend contract.
//!
//! Everything an element or the kernel needs from a UI backend, and
//! nothing more: no layout, no painting, no identity of its own. A
//! host supplies an opaque node handle and four operations over it.
//!
//! What a host does *not* supply: any notion of an interaction.
//! [`Fynix::aim`](crate::Fynix::aim) is the one primitive for pointing
//! a lane somewhere, and it takes only a node, a field, and a target:
//! nothing about when to call it. Wiring that to an event is entirely
//! the backend's own business, because the events differ by backend
//! and even within one backend there's no single "interaction" that
//! covers every trigger a style wants.

use alloc::vec::Vec;
use core::hash::Hash;

pub trait Host: Sized + Send + Sync + 'static {
    /// Opaque handle to a node. The kernel never inspects it.
    type Node: Copy + Eq + Hash + Send + Sync + 'static;

    /// State the builders read and the applies write. One type, not
    /// two: the kernel only ever holds one of `&` or `&mut` at a time.
    type World: 'static;

    /// Read-only context every [`Style::apply`](crate::style::Style::apply)
    /// gets alongside the element it writes - a theme, most often. One
    /// type for the whole host rather than per style, since a look
    /// and the palette it draws from are rarely two different
    /// questions.
    ///
    /// `Clone`, and read out of `World` by value rather than by
    /// reference: a theme that lives *in* `World` (a `Resource`, on
    /// most hosts) could never be borrowed out and held anywhere
    /// alongside `&mut World`, [`Ui`](crate::ui::Ui) included. Cloned
    /// once, into [`Fynix`](crate::Fynix)'s own field, at the top of
    /// each [`flush`](crate::Fynix::flush) - everything downstream of
    /// that borrows it from there instead, so this is the only clone
    /// there ever is. `Default` is what a fresh [`Fynix`] starts with,
    /// before its first flush ever runs.
    type Theme: Clone + Default + 'static;

    /// Seconds since the last flush, which is what a transition
    /// advances by. The kernel has no clock of its own.
    fn delta(world: &Self::World) -> f32;

    /// [`Self::Theme`], read out of `world`. Called once per flush,
    /// not once per element built - see [`Self::Theme`].
    fn theme(world: &Self::World) -> Self::Theme;

    /// Create an empty node under `parent`.
    ///
    /// The kernel spawns rather than taking a caller-made handle
    /// because cleanup depends on it: [`Host::children`] is how a
    /// rebuild finds what to despawn and whose bindings to drop. A
    /// node that never got wired to its parent would leak on every
    /// rebuild.
    fn spawn(
        world: &mut Self::World,
        parent: Self::Node,
    ) -> Self::Node;

    /// Whether `node` is still alive. The kernel outlives the nodes it
    /// watches, so it has to be able to ask.
    fn exists(world: &Self::World, node: Self::Node) -> bool;

    /// Direct children of `node`, in order.
    fn children(
        world: &Self::World,
        node: Self::Node,
    ) -> Vec<Self::Node>;

    /// Destroy `node` and everything beneath it.
    fn despawn(world: &mut Self::World, node: Self::Node);
}
