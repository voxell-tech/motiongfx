//! Colour interpolation for `#[elem(anim(...))]`.
//!
//! The travelling itself is declared on the element; this crate only
//! supplies the piece it's missing. `#[elem(anim(...))]` defaults to
//! a search for a unique `Interpolation<_>` impl on the field's
//! type, and that search only reaches impls local to this crate, to
//! the crate defining [`Interpolation`], or to the crate defining
//! the field type. `Color` and `Interpolation` are both foreign, so
//! the impl lives here, behind a marker local to this crate.

use bevy::color::Mix;
use bevy::prelude::*;
use fynix::tween::Interpolation;

/// Marks this crate's [`Interpolation`] impls for foreign types.
pub struct Marker;

impl Interpolation<Marker> for Color {
    fn interp(a: &Self, b: &Self, t: f32) -> Self {
        a.mix(b, t)
    }
}
