use crate::reactive::BevyHost;
use bevy::prelude::*;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::{Draw, Patch};

/// What a docked window fills its area with: the whole of it, and
/// scrolling if what it holds does not fit.
#[derive(Element)]
pub struct Panel {
    #[default(::Row)]
    pub direction: FlexDirection,
    /// Stretch, by default, which is what fills a docked area.
    pub align: AlignItems,
    pub justify: JustifyContent,
    pub padding: UiRect,
    /// Between rows, and between columns: a row of things wants the
    /// second, a column the first.
    #[default(Val::ZERO)]
    pub row_gap: Val,
    #[default(Val::ZERO)]
    pub column_gap: Val,
    pub scrolls: bool,
    /// How far down it is scrolled, for a panel whose scroll follows
    /// something else.
    pub scroll: f32,
}

impl Panel {
    fn node(&self) -> Node {
        Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(0),
            flex_direction: self.direction,
            align_items: self.align,
            justify_content: self.justify,
            padding: self.padding,
            row_gap: self.row_gap,
            column_gap: self.column_gap,
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
    fn build_fields(&self, element: &mut Draw<'_, BevyHost, Self>) {
        let node = element.id();
        let world = &mut *element.world;

        world.entity_mut(node).insert((
            self.node(),
            ScrollPosition(Vec2::new(0.0, self.scroll)),
        ));
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<'_, BevyHost>,
        field: PanelField,
    ) {
        let node = patch.id();
        let world = &mut *patch.world;

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
            | PanelField::RowGap
            | PanelField::ColumnGap
            | PanelField::Scrolls => {
                world.entity_mut(node).insert(self.node());
            }
        }
    }
}
