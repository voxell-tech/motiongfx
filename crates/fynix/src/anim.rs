//! Tag-driven transitions.
//!
//! A node carries a set of tags. Each animated field holds an ordered
//! list of lines, one per `on(...)`; the first whose tag is active
//! names the [`Source`] the field heads to, and the field's base names
//! it when none match. Changing a tag re-resolves the node's fields
//! and starts or redirects a [`Transition`]; [`AnimTable::tick`] plays
//! them out. See `docs/transition_tag_model.md`.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::any::TypeId;
use core::time::Duration;

use hashbrown::{HashMap, HashSet};
use typarena::type_pool::{PoolKey, TypePool};
use typarena::type_table::TypeTable;

use crate::host::Host;
use crate::lenz::FieldId;
use crate::records::{ElementTable, FieldKey};
use crate::tween::Tween;
use crate::ui::Patch;

/// What a node can be tagged with. Blanket, so any qualifying type -
/// a host enum, a marker struct - is one with nothing to write.
pub trait Tag: Copy + PartialEq + Send + Sync + 'static {}

impl<T: Copy + PartialEq + Send + Sync + 'static> Tag for T {}

/// Reads a destination value out of the element that owns it.
pub type Access<H, T> = for<'a> fn(
    &'a ElementTable<H>,
    <H as Host>::Node,
) -> Option<&'a T>;

/// Whether one `on(...)` line's tag is currently set on the node.
pub type ActiveFn<H> =
    fn(&ElementTable<H>, <H as Host>::Node) -> bool;

/// Where a field can head, and how it gets there.
pub struct Source<H: Host, T> {
    pub access: Access<H, T>,
    pub tween: Tween<T>,
    pub patch: fn(&mut Patch<H>, &T),
}

/// One field's move in progress. Absent once it settles.
struct Transition<T> {
    tween: Tween<T>,
    /// The leg's start. The only value kept: an interrupted leg begins
    /// mid-interpolation, which is in no field and no [`Source`].
    from: T,
    elapsed: Duration,
    to: PoolKey,
    /// Reverse iff the next destination is this one.
    departed: PoolKey,
}

/// One animated field: its lines in priority order, and where it
/// rests when none match.
pub struct FieldLines<H: Host> {
    field: FieldId,
    value_ty: TypeId,
    base: PoolKey,
    lines: Box<[(ActiveFn<H>, PoolKey)]>,
}

impl<H: Host> FieldLines<H> {
    /// The [`Source`] this field heads to under the node's current
    /// tags. First matching line wins; `base` when none do.
    fn resolve(
        &self,
        elements: &ElementTable<H>,
        node: H::Node,
    ) -> PoolKey {
        self.lines
            .iter()
            .find(|(active, _)| active(elements, node))
            .map_or(self.base, |&(_, key)| key)
    }
}

type TickFn<H> = fn(
    &mut TypeTable<FieldKey<H>>,
    &TypePool,
    Duration,
    &mut <H as Host>::World,
    &ElementTable<H>,
    &<H as Host>::Theme,
);

type RetargetFn<H> = fn(
    &ElementTable<H>,
    &mut TypeTable<FieldKey<H>>,
    &TypePool,
    FieldKey<H>,
    PoolKey,
    PoolKey,
);

/// Every animated field's sources, lines, and moves in progress.
pub struct AnimTable<H: Host> {
    /// [`Source`] per `(field, destination)`, shared by every instance
    /// of the element type.
    pool: TypePool,
    /// `Transition<T>` per moving field.
    rows: TypeTable<FieldKey<H>>,
    /// Every field that has ever moved, for the sweep when its node
    /// dies. Goes away with the despawn hook.
    keys: HashSet<FieldKey<H>>,
    /// One entry per value type a field has been registered for.
    ticks: Vec<(TypeId, TickFn<H>)>,
    retargets: Vec<(TypeId, RetargetFn<H>)>,
    /// Per element type, its animated fields. Written once.
    animated: HashMap<TypeId, Box<[FieldLines<H>]>>,
}

impl<H: Host> Default for AnimTable<H> {
    fn default() -> Self {
        Self {
            pool: TypePool::new(),
            rows: TypeTable::new(),
            keys: HashSet::new(),
            ticks: Vec::new(),
            retargets: Vec::new(),
            animated: HashMap::new(),
        }
    }
}

impl<H: Host> AnimTable<H> {
    /// Whether `kind` has already been registered.
    pub fn knows(&self, kind: TypeId) -> bool {
        self.animated.contains_key(&kind)
    }

    /// Register element type `kind`'s animated fields. Call once, on
    /// the first build of that type.
    pub fn register(
        &mut self,
        kind: TypeId,
        fields: impl FnOnce(&mut Registrar<'_, H>),
    ) {
        if self.knows(kind) {
            return;
        }
        let mut registrar = Registrar {
            pool: &mut self.pool,
            ticks: &mut self.ticks,
            retargets: &mut self.retargets,
            lines: Vec::new(),
        };
        fields(&mut registrar);
        let lines = registrar.lines.into_boxed_slice();
        self.animated.insert(kind, lines);
    }

    /// Re-resolve every animated field of `node` against its tags,
    /// which `edit` is free to change first.
    fn reresolve(
        &mut self,
        elements: &mut ElementTable<H>,
        kind: TypeId,
        node: H::Node,
        edit: impl FnOnce(&mut ElementTable<H>),
    ) {
        let Some(fields) = self.animated.get(&kind) else {
            return;
        };

        // Resting sources under the tags as they stand.
        let resting: Vec<PoolKey> = fields
            .iter()
            .map(|field| field.resolve(elements, node))
            .collect();

        edit(elements);

        for (field, from) in fields.iter().zip(resting) {
            let to = field.resolve(elements, node);
            let Some(retarget) = self
                .retargets
                .iter()
                .find(|(id, _)| *id == field.value_ty)
                .map(|(_, retarget)| *retarget)
            else {
                continue;
            };
            let key = FieldKey::new(node, field.field);
            retarget(
                elements, &mut self.rows, &self.pool, key, from, to,
            );
            self.keys.insert(key);
        }
    }

    /// Tag `node` with `tag`, replacing any previous tag of the same
    /// type. Other tag types are left alone.
    pub fn set_tag<T: Tag>(
        &mut self,
        elements: &mut ElementTable<H>,
        kind: TypeId,
        node: H::Node,
        tag: T,
    ) {
        self.reresolve(elements, kind, node, |elements| {
            elements.insert(node, tag);
        });
    }

    /// Drop `node`'s tag of type `T`. Its fields fall to their next
    /// active line, or to base.
    pub fn unset_tag<T: Tag>(
        &mut self,
        elements: &mut ElementTable<H>,
        kind: TypeId,
        node: H::Node,
    ) {
        self.reresolve(elements, kind, node, |elements| {
            elements.remove::<T>(&node);
        });
    }

    /// Advance every moving field by `dt`.
    pub fn tick(
        &mut self,
        dt: Duration,
        world: &mut H::World,
        elements: &ElementTable<H>,
        theme: &H::Theme,
    ) {
        for i in 0..self.ticks.len() {
            (self.ticks[i].1)(
                &mut self.rows,
                &self.pool,
                dt,
                world,
                elements,
                theme,
            );
        }
    }

    /// Drop rows whose nodes the backend no longer has.
    pub fn retain(&mut self, mut keep: impl FnMut(H::Node) -> bool) {
        let rows = &mut self.rows;
        self.keys.retain(|key| {
            let live = keep(key.node);
            if !live {
                rows.remove_row(key);
            }
            live
        });
    }

    /// How many fields are moving.
    pub fn len(&self) -> usize {
        self.keys
            .iter()
            .filter(|key| self.rows.contains_row(key))
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Collects one element type's animated fields at registration.
pub struct Registrar<'a, H: Host> {
    pool: &'a mut TypePool,
    ticks: &'a mut Vec<(TypeId, TickFn<H>)>,
    retargets: &'a mut Vec<(TypeId, RetargetFn<H>)>,
    lines: Vec<FieldLines<H>>,
}

impl<H: Host> Registrar<'_, H> {
    /// Register one animated field: the base it rests at when no line
    /// matches, then its lines, added by `add` in priority order.
    pub fn field<T>(
        &mut self,
        field: FieldId,
        patch: fn(&mut Patch<H>, &T),
        access: Access<H, T>,
        tween: Tween<T>,
        add: impl FnOnce(&mut Lines<'_, H, T>),
    ) where
        T: Clone + Send + Sync + 'static,
    {
        let value_ty = TypeId::of::<T>();
        if !self.ticks.iter().any(|(seen, _)| *seen == value_ty) {
            self.ticks.push((value_ty, tick::<H, T>));
            self.retargets.push((value_ty, retarget::<H, T>));
        }

        let base = self.pool.insert(Source {
            access,
            tween,
            patch,
        });

        let mut lines = Lines {
            pool: self.pool,
            patch,
            lines: Vec::new(),
        };
        add(&mut lines);

        self.lines.push(FieldLines {
            field,
            value_ty,
            base,
            lines: lines.lines.into_boxed_slice(),
        });
    }
}

/// One field's `on(...)` lines, collected in priority order.
pub struct Lines<'a, H: Host, T> {
    pool: &'a mut TypePool,
    patch: fn(&mut Patch<H>, &T),
    lines: Vec<(ActiveFn<H>, PoolKey)>,
}

impl<H: Host, T> Lines<'_, H, T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Add a line. Earlier calls win over later ones.
    pub fn on(
        &mut self,
        active: ActiveFn<H>,
        access: Access<H, T>,
        tween: Tween<T>,
    ) -> &mut Self {
        let key = self.pool.insert(Source {
            access,
            tween,
            patch: self.patch,
        });
        self.lines.push((active, key));
        self
    }
}

/// Advance every `Transition<T>` by `dt`, dropping the ones that
/// arrive.
fn tick<H, T>(
    rows: &mut TypeTable<FieldKey<H>>,
    pool: &TypePool,
    dt: Duration,
    world: &mut H::World,
    elements: &ElementTable<H>,
    theme: &H::Theme,
) where
    H: Host,
    T: Clone + Send + Sync + 'static,
{
    let mut done: Vec<FieldKey<H>> = Vec::new();

    for (key, transition) in rows.iter_mut::<Transition<T>>() {
        let Some(source) = pool.get::<Source<H, T>>(&transition.to)
        else {
            done.push(*key);
            continue;
        };
        let Some(to) = (source.access)(elements, key.node) else {
            done.push(*key);
            continue;
        };

        transition.elapsed += dt;
        let value = (transition.tween.lerp)(
            &transition.from,
            to,
            transition.tween.at(transition.elapsed),
        );
        let mut patch = Patch::new(world, key.node, theme);
        (source.patch)(&mut patch, &value);

        if transition.elapsed >= transition.tween.duration {
            done.push(*key);
        }
    }

    for key in done {
        rows.remove::<Transition<T>>(&key);
    }
}

/// Point `key`'s field at `to`, starting a leg or redirecting the one
/// already running.
fn retarget<H, T>(
    elements: &ElementTable<H>,
    rows: &mut TypeTable<FieldKey<H>>,
    pool: &TypePool,
    key: FieldKey<H>,
    from_key: PoolKey,
    to: PoolKey,
) where
    H: Host,
    T: Clone + Send + Sync + 'static,
{
    let Some(tween) =
        pool.get::<Source<H, T>>(&to).map(|source| source.tween)
    else {
        return;
    };

    if let Some(transition) = rows.get_mut::<Transition<T>>(&key) {
        if to == transition.to {
            return;
        }

        // Where it has reached, before the endpoints move.
        let here = pool
            .get::<Source<H, T>>(&transition.to)
            .and_then(|source| (source.access)(elements, key.node))
            .map(|target| {
                (transition.tween.lerp)(
                    &transition.from,
                    target,
                    transition.tween.at(transition.elapsed),
                )
            });
        let Some(here) = here else { return };

        let reverse = to == transition.departed;
        let spent = transition.elapsed;

        transition.departed = transition.to;
        transition.to = to;
        transition.from = here;
        transition.elapsed = Duration::ZERO;
        transition.tween = tween;
        if reverse {
            // Only as far back as it had come.
            transition.tween.duration = tween.duration.min(spent);
        }
        return;
    }

    // Settled: it rests at `from_key`, and stays there if that is
    // where the tags still point.
    if to == from_key {
        return;
    }
    let from = pool
        .get::<Source<H, T>>(&from_key)
        .and_then(|source| (source.access)(elements, key.node))
        .cloned();
    let Some(from) = from else { return };

    rows.insert(
        key,
        Transition {
            tween,
            from,
            elapsed: Duration::ZERO,
            to,
            departed: from_key,
        },
    );
}
