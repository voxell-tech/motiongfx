use crate::reactive::BevyHost;
use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::Button as ButtonBehavior;
use bevy::window::SystemCursorIcon;
use bevy_fynix::EntityExt as _;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::{Build, Patch};

use super::{ButtonElem, Label};

/// A block's header box: an absolutely positioned, bordered container
/// with a fold chevron and its label pinned to its top-left corner.
/// Every `Node::Block` in a scene's animation tree gets one of these -
/// an action leaf has no children and so no label, and stays a plain
/// `Frame` instead.
///
/// Clickable exactly like [`TimelineAction`](super::TimelineAction):
/// it carries the same [`ButtonBehavior`], so a click fires
/// `Activate` the caller can select it on. `chevron` is its own
/// absolutely-positioned [`ButtonElem`] rather than a normal-flow
/// child beside the label: a row of the two would need
/// `AlignItems::Center` to line them up, which (this box spanning the
/// block's full height, not just its header strip) centers the row in
/// the whole block instead of just the header. Positioning `chevron`
/// itself, not the label, sidesteps that without pulling it out of
/// this element altogether.
#[derive(Element)]
pub struct TimelineBlock {
    #[elem(child)]
    pub chevron: ButtonElem,
    #[elem(child)]
    pub label: Label,
    pub top: f32,
    pub left: f32,
    pub width: f32,
    pub height: f32,
    #[default(Color::NONE)]
    pub background: Color,
    #[default(Color::NONE)]
    pub border: Color,
}

impl TimelineBlock {
    fn node(&self) -> Node {
        Node {
            position_type: PositionType::Absolute,
            top: px(self.top),
            left: px(self.left),
            width: px(self.width),
            height: px(self.height),
            // The label sits in normal flow rather than absolute, so
            // this padding alone is what pins it to the top-left
            // corner - left wide enough to clear the chevron.
            padding: UiRect::new(px(16), Val::ZERO, px(2), Val::ZERO),
            border: UiRect::all(px(1)),
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for TimelineBlock {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((
            self.node(),
            BackgroundColor(self.background),
            BorderColor::all(self.border),
            ButtonBehavior,
            EntityCursor::System(SystemCursorIcon::Pointer),
        ));
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: TimelineBlockField,
    ) {
        match field {
            TimelineBlockField::Top
            | TimelineBlockField::Left
            | TimelineBlockField::Width
            | TimelineBlockField::Height => {
                patch.insert(self.node());
            }
            TimelineBlockField::Background => {
                patch.insert(BackgroundColor(self.background));
            }
            TimelineBlockField::Border => {
                patch.insert(BorderColor::all(self.border));
            }
        }
    }
}
