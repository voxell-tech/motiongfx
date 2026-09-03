//! How a widget's look moves, packaged so a widget picks one rather
//! than spelling it out. Each takes the field by cursor, so one of
//! them serves every widget with a colour in the right place.

use crate::reactive::FynixHost;
use crate::theme::EditorTheme;
use bevy::color::Mix;
use bevy::picking::events::{
    Cancel, Out, Over, Pointer, Press, Release,
};
use bevy::prelude::*;
use bevy_fynix::interact::OnExt;
use fynix::element::Element;
use fynix::host::Host;
use fynix::lenz::{Cursor, FieldPath, Identity};
use fynix::tween::Tween;
use fynix::ui::{Bindable, Build, ElementMut};
use motiongfx_interp::ease;

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

/// Lights a field under the cursor, reading its base out of the
/// kernel's own table. Only [`ElementMut`] offers this, not
/// [`Build`]: a node running its own `#[element(build = ...)]` hook
/// has no entry there yet.
pub trait MotionExt<E: Element<<Self as LitFrom<E>>::Host>>:
    LitFrom<E>
{
    /// Lights `field` under the cursor and again while held, leaving
    /// the element's own colour to show the rest of the time. The
    /// base is never written, so that is what it returns to.
    fn lit<P, T>(
        &mut self,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        hover: Color,
        press: Color,
    ) -> &mut Self
    where
        P: FieldPath<Source = E, Target = T> + Bindable<FynixHost>,
        T: Lit;

    /// Same as [`Self::lit()`], but watches a specific entity, not
    /// this node.
    fn lit_entity<P, T>(
        &mut self,
        entity: Entity,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        hover: Color,
        press: Color,
    ) -> &mut Self
    where
        P: FieldPath<Source = E, Target = T> + Bindable<FynixHost>,
        T: Lit;
}

/// As [`MotionExt`], but given the base explicitly instead of
/// reading it out of the kernel's table, for a
/// `#[element(build = ...)]` hook whose node has no entry there yet.
/// Both [`ElementMut`] and [`Build`] offer this.
pub trait LitFrom<E: Element<Self::Host>> {
    type Host: Host;

    fn theme(&self) -> &EditorTheme;

    fn lit_from<P, T>(
        &mut self,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        base: T,
        hover: Color,
        press: Color,
    ) -> &mut Self
    where
        P: FieldPath<Source = E, Target = T> + Bindable<FynixHost>,
        T: Lit;

    /// Same as [`Self::lit_from()`], but watches a specific entity,
    /// not this node.
    fn lit_entity_from<P, T>(
        &mut self,
        entity: Entity,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        base: T,
        hover: Color,
        press: Color,
    ) -> &mut Self
    where
        P: FieldPath<Source = E, Target = T> + Bindable<FynixHost>,
        T: Lit;
}

impl<E: Element<FynixHost> + Send + Sync> MotionExt<E>
    for ElementMut<'_, '_, FynixHost, E>
{
    fn lit<P, T>(
        &mut self,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        hover: Color,
        press: Color,
    ) -> &mut Self
    where
        P: FieldPath<Source = E, Target = T> + Bindable<FynixHost>,
        T: Lit,
    {
        let node = self.id();
        self.lit_entity(node, field, hover, press)
    }

    fn lit_entity<P, T>(
        &mut self,
        entity: Entity,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        hover: Color,
        press: Color,
    ) -> &mut Self
    where
        P: FieldPath<Source = E, Target = T> + Bindable<FynixHost>,
        T: Lit,
    {
        let interact_ms = self.theme().interact_ms;
        self.transition(
            field,
            Tween::ms(interact_ms, T::mix)
                .ease(ease::cubic::ease_out),
        );
        on_lit(self, entity, field, hover, press)
    }
}

impl<E: Element<FynixHost> + Send + Sync> LitFrom<E>
    for ElementMut<'_, '_, FynixHost, E>
{
    type Host = FynixHost;

    fn theme(&self) -> &EditorTheme {
        self.ui.theme
    }

    fn lit_from<P, T>(
        &mut self,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        base: T,
        hover: Color,
        press: Color,
    ) -> &mut Self
    where
        P: FieldPath<Source = E, Target = T> + Bindable<FynixHost>,
        T: Lit,
    {
        let node = self.id();
        self.lit_entity_from(node, field, base, hover, press)
    }

    fn lit_entity_from<P, T>(
        &mut self,
        entity: Entity,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        base: T,
        hover: Color,
        press: Color,
    ) -> &mut Self
    where
        P: FieldPath<Source = E, Target = T> + Bindable<FynixHost>,
        T: Lit,
    {
        let interact_ms = self.theme().interact_ms;
        self.transition_from(
            field,
            base,
            Tween::ms(interact_ms, T::mix)
                .ease(ease::cubic::ease_out),
        );
        on_lit(self, entity, field, hover, press)
    }
}

impl<E: Element<FynixHost> + Send + Sync> LitFrom<E>
    for Build<'_, FynixHost, E>
{
    type Host = FynixHost;

    fn theme(&self) -> &EditorTheme {
        self.theme
    }

    fn lit_from<P, T>(
        &mut self,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        base: T,
        hover: Color,
        press: Color,
    ) -> &mut Self
    where
        P: FieldPath<Source = E, Target = T> + Bindable<FynixHost>,
        T: Lit,
    {
        let node = self.id();
        self.lit_entity_from(node, field, base, hover, press)
    }

    fn lit_entity_from<P, T>(
        &mut self,
        entity: Entity,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        base: T,
        hover: Color,
        press: Color,
    ) -> &mut Self
    where
        P: FieldPath<Source = E, Target = T> + Bindable<FynixHost>,
        T: Lit,
    {
        let interact_ms = self.theme().interact_ms;
        self.transition_from(
            field,
            base,
            Tween::ms(interact_ms, T::mix)
                .ease(ease::cubic::ease_out),
        );
        on_lit(self, entity, field, hover, press)
    }
}

/// The pointer wiring a lit field needs: lights on hover, lights
/// harder on press, releases on out or cancel.
fn on_lit<T, E, P, Target>(
    elem: &mut T,
    entity: Entity,
    field: fn(Cursor<Identity<E>>) -> Cursor<P>,
    hover: Color,
    press: Color,
) -> &mut T
where
    T: OnExt<E, EditorTheme>,
    E: Element<FynixHost> + Send + Sync,
    P: FieldPath<Source = E, Target = Target>,
    Target: Lit,
{
    elem.on_entity::<Pointer<Over>>(entity)
        .aim(field, Some(Target::lit(hover)));
    elem.on_entity::<Pointer<Press>>(entity)
        .aim(field, Some(Target::lit(press)));
    elem.on_entity::<Pointer<Release>>(entity)
        .aim(field, Some(Target::lit(hover)));
    // `Cancel` is the drag that carried the pointer away without
    // an `Out` to go with it, and means the same thing.
    elem.on_entity::<Pointer<Out>>(entity).aim(field, None);
    elem.on_entity::<Pointer<Cancel>>(entity).aim(field, None);

    elem
}
