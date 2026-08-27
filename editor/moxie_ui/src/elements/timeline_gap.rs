use crate::reactive::BevyHost;
use bevy::prelude::*;
use bevy::ui::widget::{ImageNode, NodeImageMode};
use bevy_fynix::EntityExt as _;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::{Build, Patch};

/// The span before a node's own `delay` ends: a tiled pattern from
/// where it would have started to where it actually starts, marking
/// the gap apart from an action's fill or a block's own header.
/// Ignores the pointer - it sits directly over the scrubbable track,
/// and a click there should scrub, not stop at a decoration.
#[derive(Element)]
pub struct TimelineGap {
    pub top: f32,
    pub left: f32,
    pub width: f32,
    pub height: f32,
    pub image: Handle<Image>,
    #[default(Color::WHITE)]
    pub color: Color,
}

impl TimelineGap {
    fn node(&self) -> Node {
        Node {
            position_type: PositionType::Absolute,
            top: px(self.top),
            left: px(self.left),
            width: px(self.width),
            height: px(self.height),
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for TimelineGap {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((
            self.node(),
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

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: TimelineGapField,
    ) {
        match field {
            TimelineGapField::Top
            | TimelineGapField::Left
            | TimelineGapField::Width
            | TimelineGapField::Height => {
                patch.insert(self.node());
            }
            TimelineGapField::Image => {
                if let Some(mut image) =
                    patch.entity_mut().get_mut::<ImageNode>()
                {
                    image.image = self.image.clone();
                }
            }
            TimelineGapField::Color => {
                if let Some(mut image) =
                    patch.entity_mut().get_mut::<ImageNode>()
                {
                    image.color = self.color;
                }
            }
        }
    }
}
