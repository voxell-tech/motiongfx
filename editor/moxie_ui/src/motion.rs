//! How a widget's colour moves.
//!
//! The travelling itself is declared on the element, by
//! `#[elem(anim(...))]`; what is left here is the interpolation a
//! colour needs, since [`Color`] is foreign to both crates and cannot
//! carry fynix's own.

use bevy::color::Mix;
use bevy::prelude::*;

/// How a colour walks to another. Named by a field's
/// `#[elem(anim(lerp = <Color as Lit>::mix))]`.
pub trait Lit: Clone + PartialEq + Send + Sync + 'static {
    fn mix(from: &Self, to: &Self, t: f32) -> Self;
}

impl Lit for Color {
    fn mix(from: &Self, to: &Self, t: f32) -> Self {
        from.mix(to, t)
    }
}

/// `None` has nothing to fade from or to, so a leg touching it jumps
/// rather than guesses a colour partway to "no colour".
impl Lit for Option<Color> {
    fn mix(from: &Self, to: &Self, t: f32) -> Self {
        match (from, to) {
            (Some(from), Some(to)) => Some(from.mix(to, t)),
            _ if t >= 1.0 => *to,
            _ => *from,
        }
    }
}
