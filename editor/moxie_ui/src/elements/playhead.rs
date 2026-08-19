use crate::reactive::BevyHost;
use bevy::prelude::*;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::{Build, Patch};

const PLAYHEAD_COLOR: Color = Color::srgb(0.95, 0.30, 0.35);

/// The playhead line, positioned by the editor's playhead system.
#[derive(Element)]
pub struct PlayheadLine {
    pub left: f32,
}

impl PlayheadLine {
    fn node(&self) -> Node {
        Node {
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
        let node = build.id();
        let world = &mut *build.world;

        world.entity_mut(node).insert((
            self.node(),
            ZIndex(10),
            BackgroundColor(PLAYHEAD_COLOR),
            Pickable::IGNORE,
        ));
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: PlayheadLineField,
    ) {
        let node = patch.id();
        let world = &mut *patch.world;

        match field {
            PlayheadLineField::Left => {
                if let Some(mut node) = world.get_mut::<Node>(node) {
                    node.left = px(self.left);
                }
            }
        }
    }
}
