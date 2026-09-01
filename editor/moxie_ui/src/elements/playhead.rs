use crate::reactive::BevyHost;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::{ElementVisual, element};
use fynix::ui::{Build, Patch};

const LINE_WIDTH: f32 = 2.0;
const HEAD_WIDTH: f32 = 10.0;

/// The playhead line, positioned by the editor's playhead system.
#[element]
pub struct PlayheadLine {
    pub left: f32,
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

    fn node(&self) -> Node {
        Node {
            position_type: PositionType::Absolute,
            top: px(0),
            bottom: px(0),
            left: px(self.left),
            width: px(LINE_WIDTH),
            flex_grow: 1.0,
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for PlayheadLine {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        let color = build.theme.palette.orange;

        build.insert((
            self.node(),
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

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: PlayheadLineField,
    ) {
        match field {
            PlayheadLineField::Left => {
                if let Some(mut node) =
                    patch.entity_mut().get_mut::<Node>()
                {
                    node.left = px(self.left);
                }
            }
        }
    }
}
