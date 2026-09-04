use crate::reactive::FynixBuild;
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::ui::widget::ImageNode;
use bevy_fynix::WorldEntityMut as _;
use bevy_fynix::tag::Hovered;
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
    #[elem(patch = PatchColor, anim(
        duration = theme.motion.interact,
        ease = theme.motion.ease,
        on(Hovered, read = Self::lit),
    ))]
    pub color: Color,
    /// What `color` travels to while hovered; `None` rests. Element
    /// state: nothing draws it, only the anim line reads it.
    pub hover_color: Option<Color>,
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
    /// Where `color` heads under the cursor: the tint if one was
    /// set, otherwise its own resting colour, so nothing moves.
    fn lit(&self) -> &Color {
        match &self.hover_color {
            Some(color) => color,
            None => &self.color,
        }
    }

    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        // The handle and colour land through `image` / `color`; this
        // just gives them an `ImageNode` to write into.
        //
        // Invisible to picking: a pickable node of its own would sit
        // in front of the parent widget's hit area and swallow the
        // pointer, so the widget never sees `Over`.
        build.insert((
            ImageNode::default(),
            Pickable {
                should_block_lower: false,
                is_hoverable: false,
            },
        ));
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
