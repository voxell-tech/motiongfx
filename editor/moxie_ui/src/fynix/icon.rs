use bevy::prelude::*;
use bevy::ui::widget::ImageNode;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

/// An image at a size of its own, which is what a [`Button`] shows.
///
/// [`Button`]: super::Button
#[derive(Element, OverrideDefault, Lenz)]
pub struct Icon {
    /// Asset path.
    pub image: String,
    pub color: Color,
    #[default(Val::Px(11.0))]
    pub size: Val,
}

impl ElementVisual<BevyHost> for Icon {
    fn build_fields(&self, world: &mut World, node: Entity) {
        let image = world.load_asset(self.image.clone());

        world.entity_mut(node).insert((
            ImageNode {
                image,
                color: self.color,
                ..default()
            },
            Node {
                width: self.size,
                height: self.size,
                ..default()
            },
        ));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: IconField,
    ) {
        match field {
            IconField::Image => {
                let image = world.load_asset(self.image.clone());

                if let Some(mut node) =
                    world.get_mut::<ImageNode>(node)
                {
                    node.image = image;
                }
            }
            IconField::Color => {
                if let Some(mut image) =
                    world.get_mut::<ImageNode>(node)
                {
                    image.color = self.color;
                }
            }
            IconField::Size => {
                if let Some(mut layout) = world.get_mut::<Node>(node)
                {
                    layout.width = self.size;
                    layout.height = self.size;
                }
            }
        }
    }
}
