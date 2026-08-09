use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

/// A node the size of the window, for something that positions itself
/// against the window rather than against a parent.
#[derive(Element, OverrideDefault, Lenz)]
pub struct Overlay {
    /// Whether it swallows what lands on it. An overlay that only
    /// hosts a popup must not, or it freezes the UI beneath.
    pub blocking: bool,
    pub z: i32,
}

impl Overlay {
    fn pickable(&self) -> Pickable {
        Pickable {
            should_block_lower: self.blocking,
            is_hoverable: true,
        }
    }
}

impl ElementVisual<BevyHost> for Overlay {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            self.pickable(),
            GlobalZIndex(self.z),
        ));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: OverlayField,
    ) {
        let mut entity = world.entity_mut(node);

        match field {
            OverlayField::Blocking => {
                entity.insert(self.pickable());
            }
            OverlayField::Z => {
                entity.insert(GlobalZIndex(self.z));
            }
        }
    }
}
