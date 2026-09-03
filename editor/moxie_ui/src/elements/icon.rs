use crate::reactive::FynixBuild;
use bevy::prelude::*;
use bevy::ui::widget::ImageNode;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::patch::*;

/// An image at a size of its own, which is what a [`Button`] shows.
///
/// [`Button`]: super::Button
#[element(build = Self::build)]
pub struct Icon {
    /// Asset path.
    #[elem(patch = PatchImage)]
    pub image: String,
    #[elem(patch = PatchColor)]
    pub color: Color,
    #[elem(default = px(11), patch = PatchIconSize)]
    pub size: Val,
    /// Clockwise, in degrees.
    #[elem(patch = PatchRotation)]
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

field_patch!(PatchImage, String, |patch, v| {
    let handle = patch.world.load_asset(v.to_owned());
    with::<ImageNode>(patch, move |img| img.image = handle);
});

field_patch!(PatchColor, Color, |patch, v| {
    with::<ImageNode>(patch, |img| img.color = *v);
});

field_patch!(PatchRotation, f32, |patch, v| {
    patch.insert(icon_transform(*v));
});
