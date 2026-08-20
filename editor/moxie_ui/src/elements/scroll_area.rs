use crate::reactive::BevyHost;
use bevy::prelude::*;
use bevy::ui_widgets::ScrollArea as ScrollAreaBehavior;
use bevy_fynix::EntityExt as _;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::{Build, Patch};

/// A sized container with real, interactive scrolling - trackpad and
/// mouse-wheel input actually move it (`ScrollAreaBehavior`), not just
/// a clipped overflow a caller has to drive by hand.
#[derive(Element)]
pub struct ScrollArea {
    pub width: Val,
    pub height: Val,
    /// How much of its parent's remaining space this area claims -
    /// almost always `1.0` for one that should fill what's left.
    #[default(0.0)]
    pub flex_grow: f32,
    #[default(::Column)]
    pub direction: FlexDirection,
    pub align: AlignItems,
    pub justify: JustifyContent,
    pub padding: UiRect,
    /// Between rows, and between columns: a row of things wants the
    /// second, a column the first.
    #[default(::ZERO)]
    pub row_gap: Val,
    #[default(::ZERO)]
    pub column_gap: Val,
    #[default(::ZERO)]
    pub radius: Val,
    #[default(::NONE)]
    pub background: Color,
    #[default(true)]
    pub scroll_x: bool,
    #[default(true)]
    pub scroll_y: bool,
}

impl ScrollArea {
    fn node(&self) -> Node {
        Node {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            // `min: 0` lets the area shrink below its content instead
            // of forcing its parent to grow around it - without this
            // there is nothing left to scroll.
            min_width: px(0),
            min_height: px(0),
            flex_direction: self.direction,
            align_items: self.align,
            justify_content: self.justify,
            padding: self.padding,
            row_gap: self.row_gap,
            column_gap: self.column_gap,
            border_radius: BorderRadius::all(self.radius),
            overflow: Overflow {
                x: axis(self.scroll_x),
                y: axis(self.scroll_y),
            },
            ..default()
        }
    }
}

fn axis(scrolls: bool) -> OverflowAxis {
    if scrolls {
        OverflowAxis::Scroll
    } else {
        OverflowAxis::Visible
    }
}

impl ElementVisual<BevyHost> for ScrollArea {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((
            self.node(),
            BackgroundColor(self.background),
            ScrollAreaBehavior,
        ));
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: ScrollAreaField,
    ) {
        let node = patch.id();
        let world = &mut *patch.world;

        match field {
            ScrollAreaField::Background => {
                world
                    .entity_mut(node)
                    .insert(BackgroundColor(self.background));
            }
            // Every other field is one of `Node`'s, and writing it
            // whole is one insert rather than an arm apiece.
            ScrollAreaField::Width
            | ScrollAreaField::Height
            | ScrollAreaField::FlexGrow
            | ScrollAreaField::Direction
            | ScrollAreaField::Align
            | ScrollAreaField::Justify
            | ScrollAreaField::Padding
            | ScrollAreaField::RowGap
            | ScrollAreaField::ColumnGap
            | ScrollAreaField::Radius
            | ScrollAreaField::ScrollX
            | ScrollAreaField::ScrollY => {
                world.entity_mut(node).insert(self.node());
            }
        }
    }
}
