use bevy::prelude::*;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

use super::Label;

/// A block's header box: an absolutely positioned, bordered container
/// with its label pinned to its top-left corner. Every
/// `Node::Block` in a scene's animation tree gets one of these -
/// an action leaf has no children and so no label, and stays a plain
/// `Frame` instead.
#[derive(Element, OverrideDefault, Lenz)]
pub struct TimelineBlock {
    #[elem]
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
            // corner.
            padding: UiRect::new(px(4), Val::ZERO, px(2), Val::ZERO),
            border: UiRect::all(px(1)),
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for TimelineBlock {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert((
            self.node(),
            BackgroundColor(self.background),
            BorderColor::all(self.border),
        ));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: TimelineBlockField,
    ) {
        match field {
            TimelineBlockField::Top
            | TimelineBlockField::Left
            | TimelineBlockField::Width
            | TimelineBlockField::Height => {
                world.entity_mut(node).insert(self.node());
            }
            TimelineBlockField::Background => {
                world
                    .entity_mut(node)
                    .insert(BackgroundColor(self.background));
            }
            TimelineBlockField::Border => {
                world
                    .entity_mut(node)
                    .insert(BorderColor::all(self.border));
            }
        }
    }
}
