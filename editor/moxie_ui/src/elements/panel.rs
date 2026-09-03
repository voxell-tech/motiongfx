use crate::reactive::FynixBuild;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::patch::*;

/// What a docked window fills its area with: the whole of it, and
/// scrolling if what it holds does not fit.
#[element(build = Self::build)]
pub struct Panel {
    #[elem(patch = PatchDirection)]
    pub direction: FlexDirection,
    /// Stretch, by default, which is what fills a docked area.
    #[elem(patch = PatchAlign)]
    pub align: AlignItems,
    #[elem(patch = PatchJustify)]
    pub justify: JustifyContent,
    #[elem(patch = PatchPadding)]
    pub padding: UiRect,
    /// Between rows, and between columns: a row of things wants the
    /// second, a column the first.
    #[elem(default = Val::ZERO, patch = PatchRowGap)]
    pub row_gap: Val,
    #[elem(default = Val::ZERO, patch = PatchColumnGap)]
    pub column_gap: Val,
    #[elem(patch = PatchPanelScrolls)]
    pub scrolls: bool,
    /// How far down it is scrolled, for a panel whose scroll follows
    /// something else.
    #[elem(patch = PatchPanelScroll)]
    pub scroll: f32,
}

impl Panel {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                min_height: px(0),
                ..default()
            },
            ScrollPosition::default(),
        ));
    }
}
