use crate::reactive::BevyHost;
use bevy::prelude::*;
use bevy_fynix::EntityExt as _;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::{Build, Patch};

/// A block's header box: an absolutely positioned, bordered
/// container. Every `Node::Block` in a scene's animation tree gets
/// one of these - an action leaf has no children and stays a plain
/// `Frame` instead.
///
/// Clickable exactly like [`TimelineAction`](super::TimelineAction):
/// it carries the same [`ButtonBehavior`], so a click fires
/// `Activate` the caller can select it on.
#[derive(Element)]
pub struct TimelineBlock {
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
            border: UiRect::all(px(1)),
            align_items: AlignItems::Start,
            overflow: Overflow::clip(),
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
