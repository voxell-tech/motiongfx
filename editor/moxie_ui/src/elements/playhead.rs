use crate::reactive::FynixBuild;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::patch;

const LINE_WIDTH: f32 = 2.0;
const HEAD_WIDTH: f32 = 10.0;

/// The playhead line, positioned by the editor's playhead system.
#[element(build = Self::build)]
pub struct PlayheadLine {
    #[elem(patch = patch::left)]
    pub left: Val,
}

impl PlayheadLine {
    /// Playhead's head centred on the line
    fn head() -> Node {
        Node {
            position_type: PositionType::Absolute,
            left: px((LINE_WIDTH - HEAD_WIDTH) / 2.0),
            top: px(0),
            width: px(HEAD_WIDTH),
            height: px(12),
            border_radius: BorderRadius::all(px(2)),
            ..default()
        }
    }

    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        let color = build.theme.palette.orange;

        build.insert((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                bottom: px(0),
                width: px(LINE_WIDTH),
                flex_grow: 1.0,
                ..default()
            },
            ZIndex(10),
            BackgroundColor(color),
            Pickable::IGNORE,
        ));

        build.world.spawn((
            Self::head(),
            BackgroundColor(color),
            Pickable::IGNORE,
            ChildOf(build.id()),
        ));
    }
}
