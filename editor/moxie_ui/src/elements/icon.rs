use crate::reactive::{FynixBuild, FynixHost};
use bevy::prelude::*;
use bevy::ui::widget::ImageNode;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;
use fynix::ui::Patch;

use super::patch::{self, with};

/// An image at a size of its own, which is what a [`Button`] shows.
///
/// [`Button`]: super::Button
#[element(build = Self::build)]
pub struct Icon {
    /// Asset path.
    #[elem(patch = image)]
    pub image: String,
    #[elem(patch = color)]
    pub color: Color,
    #[elem(patch = patch::icon_size)]
    #[default(px(11))]
    pub size: Val,
    /// Clockwise, in degrees.
    #[elem(patch = rotation)]
    pub rotation: f32,
}

pub(super) fn icon_transform(rotation: f32) -> UiTransform {
    UiTransform::from_rotation(Rot2::degrees(rotation))
}

impl Icon {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        // The handle and colour land through `image` / `color`; this
        // just gives them an `ImageNode` to write into.
        build.insert(ImageNode::default());
    }
}

fn image(patch: &mut Patch<FynixHost>, image: &str) {
    let handle = patch.world.load_asset(image.to_owned());
    with::<ImageNode>(patch, move |img| img.image = handle);
}

fn color(patch: &mut Patch<FynixHost>, color: &Color) {
    with::<ImageNode>(patch, |img| img.color = *color);
}

fn rotation(patch: &mut Patch<FynixHost>, rotation: &f32) {
    patch.insert(icon_transform(*rotation));
}
