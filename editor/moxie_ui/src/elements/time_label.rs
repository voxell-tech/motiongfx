use crate::reactive::FynixBuild;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::Label;
use super::patch::*;

/// A time reading on the time axis, placing above the mark it reads
/// for.
#[element(build = Self::build)]
pub struct TimeLabel {
    #[elem(child)]
    pub label: Label,
    /// Pixels from the time axis's left edge.
    #[elem(patch = PatchTimeLabelX)]
    pub x: Val,
}

impl TimeLabel {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            Node {
                position_type: PositionType::Absolute,
                top: px(1),
                width: px(0),
                ..default()
            },
            Pickable::IGNORE,
        ));
    }
}
