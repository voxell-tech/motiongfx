#![no_std]

pub mod interpolation;

pub mod prelude {
    pub use motiongfx::prelude::*;

    pub use crate::interpolation::{
        interp_affine, interp_circle, interp_line, interp_point, interp_rect,
        interp_rounded_rect, interp_size, interp_vec2,
    };
}

pub use motiongfx;
