use crate::reactive::FynixBuild;
use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::Button as ButtonBehavior;
use bevy::window::SystemCursorIcon;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::Label;
use super::patch::*;

/// A block's header box: an absolutely positioned, bordered container
/// with its label pinned to its top-left corner, left padded to leave
/// room for a fold chevron drawn on top of it separately - a nested
/// row here would need `AlignItems::Center` to line the two up, which
/// (this box spanning the block's full height, not just its header
/// strip) centers the row in the whole block instead. Every
/// `Node::Block` in a scene's animation tree gets one of these - an
/// action leaf has no children and so no label, and stays a plain
/// `Frame` instead.
///
/// Clickable exactly like [`TimelineAction`](super::TimelineAction):
/// it carries the same [`ButtonBehavior`], so a click fires
/// `Activate` the caller can select it on.
#[element(build = Self::build)]
pub struct TimelineBlock {
    #[elem(child)]
    pub label: Label,
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
        build.insert((
            Node {
                position_type: PositionType::Absolute,
                // The label sits in normal flow rather than absolute,
                // so this padding alone is what pins it to the
                // top-left corner - left wide enough to clear the
                // chevron drawn over it.
                padding: UiRect::new(
                    px(16),
                    Val::ZERO,
                    px(2),
                    Val::ZERO,
                ),
                border: UiRect::all(px(1)),
                ..default()
            },
            ButtonBehavior,
            EntityCursor::System(SystemCursorIcon::Pointer),
        ));
    }
}
