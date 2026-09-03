use crate::reactive::FynixBuild;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::patch::*;

/// A block's header box: an absolutely positioned, bordered
/// container. Every `Node::Block` in a scene's animation tree gets
/// one of these - an action leaf has no children and stays a plain
/// `Frame` instead.
#[element(build = Self::build)]
pub struct TimelineBlock {
    #[elem(patch = PatchTop)]
    pub top: Val,
    #[elem(patch = PatchLeft)]
    pub left: Val,
    #[elem(patch = PatchWidth)]
    pub width: Val,
    #[elem(patch = PatchHeight)]
    pub height: Val,
    #[elem(patch = PatchBackground)]
    #[default(Color::NONE)]
    pub background: Color,
    #[elem(patch = PatchBorderColor)]
    #[default(Color::NONE)]
    pub border: Color,
}

impl TimelineBlock {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((Node {
            position_type: PositionType::Absolute,
            border: UiRect::all(px(1)),
            // Without this the header row stretches to the whole
            // block's height instead of sitting at its top.
            align_items: AlignItems::Start,
            overflow: Overflow::clip(),
            ..default()
        },));
    }
}
