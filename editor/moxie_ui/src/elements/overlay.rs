use crate::reactive::FynixBuild;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::patch::*;

/// A node the size of the window, for something that positions itself
/// against the window rather than against a parent.
#[element(build = Self::build)]
pub struct Overlay {
    /// Whether the pointer sees it. Off unless it is there to catch
    /// something: seen, it is the target of every press, and a press
    /// on it takes focus from whatever is underneath.
    #[elem(patch = PatchOverlayPickable)]
    pub catches: bool,
    #[elem(patch = PatchZIndex)]
    pub z: i32,
}

impl Overlay {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert(Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        });
    }
}
