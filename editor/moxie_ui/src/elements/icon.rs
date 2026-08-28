use crate::reactive::BevyHost;
use bevy::prelude::*;
use bevy::ui::widget::ImageNode;
use bevy_fynix::WorldEntityMut as _;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::{Build, Patch};

/// An image at a size of its own, which is what a [`Button`] shows.
///
/// [`Button`]: super::Button
#[derive(Element)]
pub struct Icon {
    /// Asset path.
    pub image: String,
    pub color: Color,
    #[default(px(11))]
    pub size: Val,
    /// Clockwise, in degrees.
    pub rotation: f32,
}

impl Icon {
    fn transform(&self) -> UiTransform {
        UiTransform::from_rotation(Rot2::degrees(self.rotation))
    }
}

impl ElementVisual<BevyHost> for Icon {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        let image = build.world.load_asset(self.image.clone());

        build.insert((
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
            self.transform(),
        ));
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: IconField,
    ) {
        match field {
            IconField::Image => {
                let image =
                    patch.world.load_asset(self.image.clone());

                if let Some(mut node) =
                    patch.entity_mut().get_mut::<ImageNode>()
                {
                    node.image = image;
                }
            }
            IconField::Color => {
                if let Some(mut image) =
                    patch.entity_mut().get_mut::<ImageNode>()
                {
                    image.color = self.color;
                }
            }
            IconField::Size => {
                if let Some(mut layout) =
                    patch.entity_mut().get_mut::<Node>()
                {
                    layout.width = self.size;
                    layout.height = self.size;
                }
            }
            IconField::Rotation => {
                patch.insert(self.transform());
            }
        }
    }
}
