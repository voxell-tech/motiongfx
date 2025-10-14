use bevy::{math::DVec2, prelude::*};
use bevy_vello_graphics::prelude::*;

use super::F32Lerp;

impl F32Lerp for VelloCircle {
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        VelloCircle {
            radius: f64::lerp(self.radius, rhs.radius, t as f64),
        }
    }
}

impl F32Lerp for VelloLine {
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        Self {
            p0: DVec2::lerp(self.p0, rhs.p0, t as f64),
            p1: DVec2::lerp(self.p1, rhs.p1, t as f64),
        }
    }
}

impl F32Lerp for VelloRect {
    fn f32lerp(&self, rhs: &Self, t: f32) -> Self {
        Self {
            size: DVec2::lerp(self.size, rhs.size, t as f64),
            anchor: DVec2::lerp(self.anchor, rhs.anchor, t as f64),
            radius: self.radius.f32lerp(&rhs.radius, t),
        }
    }
}
