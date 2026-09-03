use crate::reactive::FynixBuild;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::patch::*;

/// One mark on the time axis.
#[element(build = Self::build)]
pub struct TimeTick {
    /// Pixels from the time axis's left edge.
    #[elem(patch = PatchLeft)]
    pub x: Val,
    /// Grown upward from the time axis's bottom edge, so marks of
    /// different lengths share a baseline.
    #[elem(default = px(4), patch = PatchHeight)]
    pub height: Val,
    #[elem(default = theme.color.text_dim, patch = PatchBackground)]
    pub color: Color,
}

impl TimeTick {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                width: px(1),
                ..default()
            },
            Pickable::IGNORE,
        ));
    }
}
