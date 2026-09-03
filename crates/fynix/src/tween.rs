//! A value laid over a field while it transitions, kept beside the
//! element rather than in it.
//!
//! The element carries the *base*, the cascade's own value. A tween
//! carries what the backend is showing and writes it every frame,
//! straight through the field's `#[elem(patch = ...)]` tag.

use alloc::vec::Vec;
use core::any::TypeId;

use hashbrown::HashSet;
use typarena::type_table::TypeTable;

use crate::host::Host;
use crate::lenz::FieldId;
use crate::records::FieldKey;
use crate::transition::Transition;
use crate::ui::Patch;

/// One field's transition in progress.
pub(crate) struct Tween<H: Host, T> {
    /// The field's `#[elem(patch = ...)]` writer.
    write: fn(&mut Patch<H>, &T),
    curve: Transition<T>,
    /// The cascade's own value. A binding on the same field moves it
    /// through [`rebase`](Self::rebase).
    base: T,
    /// Where it is aimed, or `None` while heading home to the base.
    target: Option<T>,
    /// The current leg's start.
    from: T,
    elapsed: f32,
}

impl<H: Host, T: Clone> Tween<H, T> {
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

    /// The cascade value moved. Start a fresh leg if that is where it
    /// is heading.
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

/// Advances every [`Tween<H, T>`] in the table's column for one `T`.
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
    for (key, tween) in table.iter_mut::<Tween<H, T>>() {
        let node = key.node;
        tween.advance(dt, world, node, theme);
    }
}

/// Every field with a tween over it. One per field - a second tween on
/// the same field replaces rather than doubles.
///
/// A column per value type in `table`, so a frame advances one
/// contiguous run per type rather than chasing a boxed trait object
/// per field. `keys` is the set to sweep; `ticks` holds one advance
/// function per value type seen.
pub struct TweenTable<H: Host> {
    table: TypeTable<FieldKey<H>>,
    keys: HashSet<FieldKey<H>>,
    ticks: Vec<(TypeId, TickFn<H>)>,
}

impl<H: Host> Default for TweenTable<H> {
    fn default() -> Self {
        Self {
            table: TypeTable::new(),
            keys: HashSet::new(),
            ticks: Vec::new(),
        }
    }
}

impl<H: Host> TweenTable<H> {
    pub(crate) fn insert<T>(
        &mut self,
        node: H::Node,
        key: FieldId,
        tween: Tween<H, T>,
    ) where
        T: Clone + Send + Sync + 'static,
    {
        let id = TypeId::of::<T>();
        if !self.ticks.iter().any(|(seen, _)| *seen == id) {
            self.ticks.push((id, tick::<H, T>));
        }

        let key = FieldKey::new(node, key);
        // Drop any tween already on this field, whatever its type.
        self.table.remove_row(&key);
        self.table.insert(key, tween);
        self.keys.insert(key);
    }

    /// The tween on `(node, key)` as the `Tween<H, T>` it must be.
    /// `None` if there is no tween there.
    pub(crate) fn running<T: 'static>(
        &mut self,
        node: H::Node,
        key: FieldId,
    ) -> Option<&mut Tween<H, T>> {
        self.table.get_mut::<Tween<H, T>>(&FieldKey::new(node, key))
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

    /// Advance every tween by `dt`.
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

/// Lay a tween over `key` on `node`, starting from `base`.
pub(crate) fn insert_tween<H, T>(
    tweens: &mut TweenTable<H>,
    node: H::Node,
    key: FieldId,
    write: fn(&mut Patch<H>, &T),
    base: T,
    curve: Transition<T>,
) where
    H: Host,
    T: Clone + Send + Sync + 'static,
{
    tweens.insert(
        node,
        key,
        Tween {
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
