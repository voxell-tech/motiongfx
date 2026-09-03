//! A field's value while it moves to a new one, kept beside the
//! element rather than in it.
//!
//! The element carries the *base*, the cascade's own value. A
//! transition carries what the backend is showing and writes it every
//! frame, straight through the field's `#[elem(patch = ...)]` tag. Its
//! shape comes from a [`Tween`].

use alloc::vec::Vec;
use core::any::TypeId;

use hashbrown::HashSet;
use typarena::type_table::TypeTable;

use crate::host::Host;
use crate::lenz::FieldId;
use crate::records::FieldKey;
use crate::tween::Tween;
use crate::ui::Patch;

/// One field's move in progress.
pub(crate) struct Transition<H: Host, T> {
    /// The field's `#[elem(patch = ...)]` writer.
    write: fn(&mut Patch<H>, &T),
    curve: Tween<T>,
    /// The cascade's own value. Released, the transition heads back
    /// here.
    base: T,
    /// The aim, or `None` while heading home to the base.
    target: Option<T>,
    /// The current leg's start.
    from: T,
    elapsed: f32,
}

impl<H: Host, T: Clone> Transition<H, T> {
    fn heading(&self) -> &T {
        self.target.as_ref().unwrap_or(&self.base)
    }

    fn shown(&self) -> T {
        if self.curve.done(self.elapsed) {
            self.heading().clone()
        } else {
            (self.curve.lerp)(
                &self.from,
                self.heading(),
                self.curve.at(self.elapsed),
            )
        }
    }

    /// Aim at `target`, or `None` to release it back to the base.
    pub(crate) fn aim(&mut self, target: Option<T>) {
        // Snapshot before the endpoint moves.
        self.from = self.shown();
        self.elapsed = 0.0;
        self.target = target;
    }

    /// Move the base to `base`, restarting the leg if the transition
    /// is heading there.
    pub(crate) fn rebase(&mut self, base: &T) {
        if self.target.is_none() {
            self.from = self.shown();
            self.elapsed = 0.0;
        }
        self.base = base.clone();
    }

    /// Advance by `dt` and write what it reached.
    fn advance(
        &mut self,
        dt: f32,
        world: &mut H::World,
        node: H::Node,
        theme: &H::Theme,
    ) {
        // Arrived home: nothing to write, the base shows.
        if self.target.is_none() && self.curve.done(self.elapsed) {
            return;
        }

        self.elapsed += dt;
        let shown = self.shown();

        // Written every frame, even unmoved, so a binding that wrote
        // the base earlier this flush cannot win.
        let mut patch = Patch::new(world, node, theme);
        (self.write)(&mut patch, &shown);
    }
}

/// The per-type advance step in a [`TransitionTable`]'s tick registry.
type TickFn<H> = fn(
    &mut TypeTable<FieldKey<H>>,
    f32,
    &mut <H as Host>::World,
    &<H as Host>::Theme,
);

fn tick<H, T>(
    table: &mut TypeTable<FieldKey<H>>,
    dt: f32,
    world: &mut H::World,
    theme: &H::Theme,
) where
    H: Host,
    T: Clone + Send + Sync + 'static,
{
    for (key, transition) in table.iter_mut::<Transition<H, T>>() {
        let node = key.node;
        transition.advance(dt, world, node, theme);
    }
}

/// Every field with a transition over it, at most one per field.
///
/// One `Transition<H, T>` column per value type; `advance` walks each
/// column through its matching `ticks` entry.
pub struct TransitionTable<H: Host> {
    table: TypeTable<FieldKey<H>>,
    /// The rows to sweep when their nodes die.
    keys: HashSet<FieldKey<H>>,
    /// One advance step per value type a transition has been inserted
    /// for.
    ticks: Vec<(TypeId, TickFn<H>)>,
}

impl<H: Host> Default for TransitionTable<H> {
    fn default() -> Self {
        Self {
            table: TypeTable::new(),
            keys: HashSet::new(),
            ticks: Vec::new(),
        }
    }
}

impl<H: Host> TransitionTable<H> {
    pub(crate) fn insert<T>(
        &mut self,
        node: H::Node,
        key: FieldId,
        transition: Transition<H, T>,
    ) where
        T: Clone + Send + Sync + 'static,
    {
        let id = TypeId::of::<T>();
        if !self.ticks.iter().any(|(seen, _)| *seen == id) {
            self.ticks.push((id, tick::<H, T>));
        }

        let key = FieldKey::new(node, key);
        // Drop any transition already on this field, whatever its
        // value type.
        self.table.remove_row(&key);
        self.table.insert(key, transition);
        self.keys.insert(key);
    }

    /// The transition on `(node, key)`, if one is running with value
    /// type `T`.
    pub(crate) fn running<T: 'static>(
        &mut self,
        node: H::Node,
        key: FieldId,
    ) -> Option<&mut Transition<H, T>> {
        self.table
            .get_mut::<Transition<H, T>>(&FieldKey::new(node, key))
    }

    pub(crate) fn retain(
        &mut self,
        mut keep: impl FnMut(H::Node) -> bool,
    ) {
        let table = &mut self.table;
        self.keys.retain(|key| {
            let live = keep(key.node);
            if !live {
                table.remove_row(key);
            }
            live
        });
    }

    /// Advance every transition by `dt`.
    pub(crate) fn advance(
        &mut self,
        dt: f32,
        world: &mut H::World,
        theme: &H::Theme,
    ) {
        for i in 0..self.ticks.len() {
            (self.ticks[i].1)(&mut self.table, dt, world, theme);
        }
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

/// Lay a transition over `key` on `node`, starting from `base`.
pub(crate) fn insert_transition<H, T>(
    transitions: &mut TransitionTable<H>,
    node: H::Node,
    key: FieldId,
    write: fn(&mut Patch<H>, &T),
    base: T,
    curve: Tween<T>,
) where
    H: Host,
    T: Clone + Send + Sync + 'static,
{
    transitions.insert(
        node,
        key,
        Transition {
            write,
            from: base.clone(),
            base,
            target: None,
            // Settled on the base until something aims it.
            elapsed: curve.duration,
            curve,
        },
    );
}
