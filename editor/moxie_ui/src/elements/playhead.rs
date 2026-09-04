use crate::reactive::FynixBuild;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use core::f32::consts::FRAC_1_SQRT_2;
use fynix::element::element;

use super::patch;

const LINE_WIDTH: f32 = 2.0;
/// The head's square, before it is turned.
const HEAD_SIDE: f32 = 12.0;
/// Half the turned square's diagonal: how far it spreads and hangs.
const HEAD_REACH: f32 = HEAD_SIDE * FRAC_1_SQRT_2;

/// The playhead line, positioned by the editor's playhead system.
#[element(build = Self::build)]
pub struct PlayheadLine {
    #[elem(patch = patch::left)]
    pub left: Val,
    #[elem(patch = patch::top)]
    pub top: Val,
}

impl PlayheadLine {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        let color = build.theme.palette.orange;

        build.insert((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                width: px(LINE_WIDTH),
                ..default()
            },
            ZIndex(10),
            BackgroundColor(color),
            Pickable::IGNORE,
        ));

        build.world.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(LINE_WIDTH / 2.0 - HEAD_REACH),
                top: px(-HEAD_REACH),
                width: px(HEAD_REACH * 2.0),
                height: px(HEAD_REACH),
                overflow: Overflow::clip(),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(build.id()),
            children![(
                Node {
                    position_type: PositionType::Absolute,
                    left: px(HEAD_REACH - HEAD_SIDE / 2.0),
                    top: px(-HEAD_SIDE / 2.0),
                    width: px(HEAD_SIDE),
                    height: px(HEAD_SIDE),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::degrees(45.0)),
                BackgroundColor(color),
                Pickable::IGNORE,
            )],
        ));
    }
}
