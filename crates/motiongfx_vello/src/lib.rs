pub use bevy_vello_graphics;

use bevy::{math::DVec2, prelude::*};
use bevy_vello_graphics::prelude::*;
use motiongfx_core::{sequence::update_component, UpdateSequenceSet};

pub mod motion;
// pub mod svg;

pub mod prelude {
    pub use crate::motion::{fill_motion::FillMotion, stroke_motion::StrokeMotion};

    pub use bevy_vello_graphics::prelude::*;
}

pub struct MotionGfxVelloPlugin;

impl Plugin for MotionGfxVelloPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(VelloGraphicsPlugin).add_systems(
            Update,
            (
                // Fill & Stroke
                update_component::<Fill, Brush>,
                update_component::<Stroke, Brush>,
                update_component::<Stroke, f64>,
                // VelloCircle
                update_component::<VelloCircle, VelloCircle>,
                update_component::<VelloCircle, f64>,
                // VelloRect
                update_component::<VelloRect, VelloRect>,
                update_component::<VelloRect, DVec2>,
                update_component::<VelloRect, f64>,
                // VelloLine
                update_component::<VelloLine, VelloLine>,
                update_component::<VelloLine, DVec2>,
                update_component::<VelloLine, f64>,
                // VelloBezPath
                update_component::<VelloBezPath, f32>,
            )
                .in_set(UpdateSequenceSet),
        );
    }
}
