use crate::reactive::BevyHost;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::{Build, Patch};

/// The playhead line, positioned by the editor's playhead system.
#[derive(Element)]
pub struct PlayheadLine {
    pub left: f32,
}

impl PlayheadLine {
    fn node(&self) -> Node {
        Node {
            position_type: PositionType::Absolute,
            top: px(0),
            bottom: px(0),
            left: px(self.left),
            width: px(2),
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
