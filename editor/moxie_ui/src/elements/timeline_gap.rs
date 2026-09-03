use crate::reactive::FynixBuild;
use bevy::prelude::*;
use bevy::ui::widget::{ImageNode, NodeImageMode};
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::patch::*;

/// The span before a node's own `delay` ends: a tiled pattern from
/// where it would have started to where it actually starts, marking
/// the gap apart from an action's fill or a block's own header.
/// Ignores the pointer - it sits directly over the scrubbable track,
/// and a click there should scrub, not stop at a decoration.
#[element(build = Self::build)]
pub struct TimelineGap {
    #[elem(patch = PatchTop)]
    pub top: Val,
    #[elem(patch = PatchLeft)]
    pub left: Val,
    #[elem(patch = PatchWidth)]
    pub width: Val,
    #[elem(patch = PatchHeight)]
    pub height: Val,
    #[elem(patch = PatchGapImage)]
    pub image: Handle<Image>,
    #[elem(default = theme.color.text_dim, patch = PatchGapColor)]
    pub color: Color,
}

impl TimelineGap {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            ImageNode {
                image: self.image.clone(),
                color: self.color,
                image_mode: NodeImageMode::Tiled {
                    tile_x: true,
                    tile_y: true,
                    stretch_value: 1.0,
                },
                ..default()
            },
            Pickable::IGNORE,
        ));
    }
}

field_patch!(PatchGapImage, Handle<Image>, |patch, v| {
    with::<ImageNode>(patch, |image| image.image = v.clone());
});

field_patch!(PatchGapColor, Color, |patch, v| {
    with::<ImageNode>(patch, |image| image.color = *v);
});
