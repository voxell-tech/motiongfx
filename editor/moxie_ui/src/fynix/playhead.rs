use bevy::prelude::*;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

const PLAYHEAD_COLOR: Color = Color::srgb(0.95, 0.30, 0.35);

/// The playhead line, positioned by the editor's playhead system.
#[derive(Element, OverrideDefault, Lenz)]
pub struct PlayheadLine {
    pub left: f32,
}

impl PlayheadLine {
    fn node(&self) -> Node {
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            left: Val::Px(self.left),
            width: Val::Px(2.0),
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for PlayheadLine {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert((
            self.node(),
            ZIndex(10),
            BackgroundColor(PLAYHEAD_COLOR),
        ));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: PlayheadLineField,
    ) {
        match field {
            PlayheadLineField::Left => {
                if let Some(mut node) = world.get_mut::<Node>(node) {
                    node.left = Val::Px(self.left);
                }
            }
        }
    }
}
