use crate::reactive::FynixBuild;
use bevy::prelude::*;
use bevy::ui_widgets::ScrollArea as ScrollAreaBehavior;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::patch::*;

/// A sized container with real, interactive scrolling - trackpad and
/// mouse-wheel input actually move it (`ScrollAreaBehavior`), not just
/// a clipped overflow a caller has to drive by hand.
#[element(build = Self::build)]
pub struct ScrollArea {
    #[elem(patch = PatchWidth)]
    pub width: Val,
    #[elem(patch = PatchHeight)]
    pub height: Val,
    /// How much of its parent's remaining space this area claims -
    /// almost always `1.0` for one that should fill what's left.
    #[elem(patch = PatchFlexGrow)]
    #[default(0.0)]
    pub flex_grow: f32,
    #[elem(patch = PatchDirection)]
    #[default(::Column)]
    pub direction: FlexDirection,
    #[elem(patch = PatchAlign)]
    pub align: AlignItems,
    #[elem(patch = PatchJustify)]
    pub justify: JustifyContent,
    #[elem(patch = PatchPadding)]
    pub padding: UiRect,
    /// Between rows, and between columns: a row of things wants the
    /// second, a column the first.
    #[elem(patch = PatchRowGap)]
    #[default(::ZERO)]
    pub row_gap: Val,
    #[elem(patch = PatchColumnGap)]
    #[default(::ZERO)]
    pub column_gap: Val,
    #[elem(patch = PatchRadius)]
    #[default(::ZERO)]
    pub radius: Val,
    #[elem(patch = PatchBackground)]
    #[default(::NONE)]
    pub background: Color,
    #[elem(patch = PatchScrollX)]
    #[default(true)]
    pub scroll_x: bool,
    #[elem(patch = PatchScrollY)]
    #[default(true)]
    pub scroll_y: bool,
}

impl ScrollArea {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        // `min: 0` lets the area shrink below its content instead of
        // forcing its parent to grow around it - without it there is
        // nothing left to scroll.
        build.insert((
            Node {
                min_width: px(0),
                min_height: px(0),
                ..default()
            },
            ScrollAreaBehavior,
        ));
    }
}
