use crate::reactive::FynixBuild;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::patch;

/// The playhead line, positioned by the editor's playhead system.
#[element(build = Self::build)]
pub struct PlayheadLine {
    #[elem(patch = patch::left)]
    pub left: Val,
}

impl PlayheadLine {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        let color = build.theme.palette.orange;

        build.insert((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                bottom: px(0),
                width: px(2),
                flex_grow: 1.0,
                ..default()
            },
            ZIndex(10),
            BackgroundColor(color),
            Pickable::IGNORE,
        ));
    }
}
