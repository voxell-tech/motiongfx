use bevy::prelude::*;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

/// What a docked window fills its area with: the whole of it, and
/// scrolling if what it holds does not fit.
#[derive(Element, OverrideDefault, Lenz)]
pub struct Panel {
    #[default(::Row)]
    pub direction: FlexDirection,
    /// Stretch, by default, which is what fills a docked area.
    pub align: AlignItems,
    pub justify: JustifyContent,
    pub padding: UiRect,
    #[default(Val::ZERO)]
    pub gap: Val,
    pub scrolls: bool,
    /// How far down it is scrolled, for a panel whose scroll follows
    /// something else.
    pub scroll: f32,
}

impl Panel {
    fn node(&self) -> Node {
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            flex_direction: self.direction,
            align_items: self.align,
            justify_content: self.justify,
            padding: self.padding,
            row_gap: self.gap,
            column_gap: self.gap,
            overflow: if self.scrolls {
                Overflow::scroll_y()
            } else {
                Overflow::clip()
            },
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for Panel {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert((
            self.node(),
            ScrollPosition(Vec2::new(0.0, self.scroll)),
        ));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: PanelField,
    ) {
        match field {
            PanelField::Scroll => {
                if let Some(mut scroll) =
                    world.get_mut::<ScrollPosition>(node)
                {
                    scroll.0.y = self.scroll;
                }
            }
            PanelField::Direction
            | PanelField::Align
            | PanelField::Justify
            | PanelField::Padding
            | PanelField::Gap
            | PanelField::Scrolls => {
                world.entity_mut(node).insert(self.node());
            }
        }
    }
}
