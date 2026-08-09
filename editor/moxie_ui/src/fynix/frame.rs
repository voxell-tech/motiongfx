use bevy::prelude::*;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

/// The sized, optionally filled container almost every other widget's
/// root node turns out to be.
#[derive(Element, OverrideDefault, Lenz)]
pub struct Frame {
    pub width: Val,
    pub height: Val,
    pub direction: FlexDirection,
    pub align: AlignItems,
    pub justify: JustifyContent,
    pub padding: UiRect,
    #[default(Val::ZERO)]
    pub gap: Val,
    #[default(Val::ZERO)]
    pub radius: Val,
    /// Transparent by default, for a frame that only wants the
    /// layout.
    #[default(Color::NONE)]
    pub background: Color,
}

impl Frame {
    fn node(&self) -> Node {
        Node {
            width: self.width,
            height: self.height,
            flex_direction: self.direction,
            align_items: self.align,
            justify_content: self.justify,
            padding: self.padding,
            column_gap: self.gap,
            row_gap: self.gap,
            border_radius: BorderRadius::all(self.radius),
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for Frame {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world
            .entity_mut(node)
            .insert((self.node(), BackgroundColor(self.background)));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: FrameField,
    ) {
        match field {
            FrameField::Background => {
                world
                    .entity_mut(node)
                    .insert(BackgroundColor(self.background));
            }
            // Every other field is one of `Node`'s, and writing the
            // node whole is one insert rather than eight arms that
            // each write a field of it.
            FrameField::Width
            | FrameField::Height
            | FrameField::Direction
            | FrameField::Align
            | FrameField::Justify
            | FrameField::Padding
            | FrameField::Gap
            | FrameField::Radius => {
                world.entity_mut(node).insert(self.node());
            }
        }
    }
}
