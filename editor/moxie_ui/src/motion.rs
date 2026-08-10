//! How a widget's look moves, packaged so a widget picks one rather
//! than spelling it out. Each takes the field by cursor, so one of
//! them serves every widget with a colour in the right place.

use bevy::color::Mix;
use bevy::picking::events::{
    Cancel, Out, Over, Pointer, Press, Release,
};
use bevy::prelude::*;
use bevy_fynix::host::BevyHost;
use bevy_fynix::interact::OnExt;
use fynix_mock::element::Element;
use fynix_mock::host::Host;
use fynix_mock::lenz::{Cursor, FieldPath, Identity};
use fynix_mock::transition::{Transition, ease};
use fynix_mock::ui::ElementMut;

/// Long enough to read as a fade, short enough to feel immediate.
const INTERACT_MS: u32 = 120;

/// What a surface lights up to under the cursor, and while held.
pub const HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.14);
pub const PRESS: Color = Color::srgba(1.0, 1.0, 1.0, 0.22);

pub trait MotionExt<E: Element<Self::Host>>: Sized {
    type Host: Host;

    /// Lights `field` under the cursor and again while held, leaving
    /// the element's own colour to show the rest of the time. The
    /// base is never written, so that is what it returns to.
    fn lit<P>(
        self,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        hover: Color,
        press: Color,
    ) -> Self
    where
        P: FieldPath<Source = E, Target = Color>;
}

impl<E: Element<BevyHost> + Send + Sync> MotionExt<E>
    for ElementMut<'_, '_, BevyHost, E>
{
    type Host = BevyHost;

    fn lit<P>(
        self,
        field: fn(Cursor<Identity<E>>) -> Cursor<P>,
        hover: Color,
        press: Color,
    ) -> Self
    where
        P: FieldPath<Source = E, Target = Color>,
    {
        let mut elem = self.transition(
            field,
            Transition::ms(INTERACT_MS, mix).ease(ease::cubic_out),
        );

        elem.on::<Pointer<Over>>().aim(field, Some(hover));
        elem.on::<Pointer<Press>>().aim(field, Some(press));
        elem.on::<Pointer<Release>>().aim(field, Some(hover));
        // `Cancel` is the drag that carried the pointer away without
        // an `Out` to go with it, and means the same thing.
        elem.on::<Pointer<Out>>().aim(field, None);
        elem.on::<Pointer<Cancel>>().aim(field, None);

        elem
    }
}

/// Straight through, which for two tints of one surface is what the
/// eye expects.
fn mix(from: &Color, to: &Color, t: f32) -> Color {
    from.mix(to, t)
}
