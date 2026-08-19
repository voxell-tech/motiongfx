//! How a widget's look moves, packaged so a widget picks one rather
//! than spelling it out. Each takes the field by cursor, so one of
//! them serves every widget with a colour in the right place.

use crate::reactive::BevyHost;
use bevy::color::Mix;
use bevy::picking::events::{
    Cancel, Out, Over, Pointer, Press, Release,
};
use bevy::prelude::*;
use bevy_fynix::interact::OnExt as _;
use fynix_mock::element::Element;
use fynix_mock::host::Host;
use fynix_mock::lenz::{Cursor, FieldPath, Identity};
use fynix_mock::transition::Transition;
use fynix_mock::ui::ElementMut;
use motiongfx_interp::ease;

/// Long enough to read as a fade, short enough to feel immediate.
const INTERACT_MS: u32 = 120;

/// What a surface lights up to under the cursor, and while held.
pub const HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.14);
pub const PRESS: Color = Color::srgba(1.0, 1.0, 1.0, 0.22);

/// What `lit` can aim at: a colour itself, or a field that only wears
/// one sometimes.
pub trait Lit: Clone + PartialEq + Send + Sync + 'static {
    fn lit(color: Color) -> Self;
    fn mix(from: &Self, to: &Self, t: f32) -> Self;
}

impl Lit for Color {
    fn lit(color: Color) -> Self {
        color
    }

    fn mix(from: &Self, to: &Self, t: f32) -> Self {
        from.mix(to, t)
    }
}

/// `None` has nothing to fade from or to, so a leg touching it jumps
/// rather than guesses a colour partway to "no colour".
impl Lit for Option<Color> {
    fn lit(color: Color) -> Self {
        Some(color)
    }

    fn mix(from: &Self, to: &Self, t: f32) -> Self {
        match (from, to) {
            (Some(from), Some(to)) => Some(from.mix(to, t)),
            _ if t >= 1.0 => *to,
            _ => *from,
        }
    }
}

pub trait MotionExt<E: Element<Self::Host>>: Sized {
    type Host: Host;

    /// Lights `field` under the cursor and again while held, leaving
    /// the element's own colour to show the rest of the time. The
    /// base is never written, so that is what it returns to.
    fn lit<P, T>(
        self,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        hover: Color,
        press: Color,
    ) -> Self
    where
        P: FieldPath<Source = E, Target = T>,
        T: Lit;

    /// Same as [`Self::lit()`] but watching a specific entity rather
    /// than this node.
    fn lit_entity<P, T>(
        self,
        entity: Entity,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        hover: Color,
        press: Color,
    ) -> Self
    where
        P: FieldPath<Source = E, Target = T>,
        T: Lit;

    /// As [`Self::lit()`], but for
    /// [`build_fields`](fynix_mock::element::ElementVisual::build_fields):
    /// this node's element is not in the kernel's own table yet to
    /// read a base from, so `base` is passed straight through instead
    /// - `build_fields` already has it, as `&self`.
    fn lit_from<P, T>(
        self,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        base: T,
        hover: Color,
        press: Color,
    ) -> Self
    where
        P: FieldPath<Source = E, Target = T>,
        T: Lit;

    /// Same as [`Self::lit_from()`] but watching a specific entity
    /// rather than this node.
    fn lit_entity_from<P, T>(
        self,
        entity: Entity,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        base: T,
        hover: Color,
        press: Color,
    ) -> Self
    where
        P: FieldPath<Source = E, Target = T>,
        T: Lit;
}

impl<E: Element<BevyHost> + Send + Sync> MotionExt<E>
    for ElementMut<'_, '_, BevyHost, E>
{
    type Host = BevyHost;

    fn lit<P, T>(
        self,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        hover: Color,
        press: Color,
    ) -> Self
    where
        P: FieldPath<Source = E, Target = T>,
        T: Lit,
    {
        let node = self.id();
        self.lit_entity(node, field, hover, press)
    }

    fn lit_entity<P, T>(
        self,
        entity: Entity,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        hover: Color,
        press: Color,
    ) -> Self
    where
        P: FieldPath<Source = E, Target = T>,
        T: Lit,
    {
        let elem = self.transition(
            field,
            Transition::ms(INTERACT_MS, T::mix)
                .ease(ease::cubic::ease_out),
        );
        on_lit(elem, entity, field, hover, press)
    }

    fn lit_from<P, T>(
        self,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        base: T,
        hover: Color,
        press: Color,
    ) -> Self
    where
        P: FieldPath<Source = E, Target = T>,
        T: Lit,
    {
        let node = self.id();
        self.lit_entity_from(node, field, base, hover, press)
    }

    fn lit_entity_from<P, T>(
        self,
        entity: Entity,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        base: T,
        hover: Color,
        press: Color,
    ) -> Self
    where
        P: FieldPath<Source = E, Target = T>,
        T: Lit,
    {
        let elem = self.transition_from(
            field,
            base,
            Transition::ms(INTERACT_MS, T::mix)
                .ease(ease::cubic::ease_out),
        );
        on_lit(elem, entity, field, hover, press)
    }
}

/// The pointer wiring [`MotionExt::lit_entity`] and
/// [`MotionExt::lit_entity_from`] share - only how the lane's base is
/// found differs between them.
fn on_lit<'u, 'a, E, P, T>(
    mut elem: ElementMut<'u, 'a, BevyHost, E>,
    entity: Entity,
    field: fn(Cursor<Identity<E>>) -> Cursor<P>,
    hover: Color,
    press: Color,
) -> ElementMut<'u, 'a, BevyHost, E>
where
    E: Element<BevyHost> + Send + Sync,
    P: FieldPath<Source = E, Target = T>,
    T: Lit,
{
    elem.on_entity::<Pointer<Over>>(entity)
        .aim(field, Some(T::lit(hover)));
    elem.on_entity::<Pointer<Press>>(entity)
        .aim(field, Some(T::lit(press)));
    elem.on_entity::<Pointer<Release>>(entity)
        .aim(field, Some(T::lit(hover)));
    // `Cancel` is the drag that carried the pointer away without
    // an `Out` to go with it, and means the same thing.
    elem.on_entity::<Pointer<Out>>(entity).aim(field, None);
    elem.on_entity::<Pointer<Cancel>>(entity).aim(field, None);

    elem
}
