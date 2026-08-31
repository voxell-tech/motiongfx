//! A leaf's tab bar: the strip, the scrolling row, and one tab.

use crate::reactive::BevyHost;
use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::window::SystemCursorIcon;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::{ElementVisual, element};
use fynix::ui::{Build, Patch};

use super::{ButtonElem, Icon, Label};
use crate::widgets::dock::{DockTab, DockTabRow, TAB_HEIGHT, TabId};

/// The strip across the top of a leaf.
#[element]
pub struct TabBar {
    #[default(Color::srgba(1.0, 1.0, 1.0, 0.03))]
    pub background: Color,
}

impl ElementVisual<BevyHost> for TabBar {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                width: percent(100),
                height: px(TAB_HEIGHT),
                // No left padding: the first tab sits flush.
                padding: UiRect::new(
                    Val::ZERO,
                    px(8),
                    px(1),
                    Val::ZERO,
                ),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(self.background),
        ));
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: TabBarField,
    ) {
        match field {
            TabBarField::Background => {
                patch.insert(BackgroundColor(self.background));
            }
        }
    }
}

/// What the tabs themselves sit in, which scrolls when they overflow.
#[element]
pub struct TabRow;

impl ElementVisual<BevyHost> for TabRow {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((
            DockTabRow,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(2),
                height: percent(100),
                overflow: Overflow::scroll_x(),
                flex_shrink: 1.0,
                min_width: px(0),
                ..default()
            },
            ScrollPosition::default(),
        ));
    }

    fn patch_fields(
        &self,
        _patch: &mut Patch<BevyHost>,
        field: TabRowField,
    ) {
        match field {}
    }
}

/// One tab: its icon, its name, and the button that closes it.
///
/// Which tab is active is a field rather than a rebuild, because
/// switching tabs must not take the drag in progress or the row's
/// scroll offset with it.
#[element]
pub struct Tab {
    #[elem(child)]
    pub icon: Option<Icon>,
    #[elem(child)]
    pub label: Label,
    #[elem(child)]
    pub close: Option<ButtonElem>,
    pub window_id: String,
    #[default(TabId(0))]
    pub tab: TabId,
    pub active: bool,
    #[default(Color::srgba(1.0, 1.0, 1.0, 0.06))]
    pub fill: Color,
}

impl Tab {
    fn background(&self) -> BackgroundColor {
        BackgroundColor(if self.active {
            self.fill
        } else {
            Color::NONE
        })
    }
}

impl ElementVisual<BevyHost> for Tab {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((
            DockTab {
                window_id: self.window_id.clone(),
                tab_id: self.tab,
            },
            // A tab is dragged, so it says so at rest. The drag
            // itself swaps in `Grabbing` through `OverrideCursor`.
            EntityCursor::System(SystemCursorIcon::Grab),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                padding: UiRect::axes(px(8), px(3)),
                height: percent(100),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            self.background(),
        ));
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: TabField,
    ) {
        match field {
            TabField::Active | TabField::Fill => {
                patch.insert(self.background());
            }
            TabField::WindowId | TabField::Tab => {
                patch.insert(DockTab {
                    window_id: self.window_id.clone(),
                    tab_id: self.tab,
                });
            }
        }
    }
}
