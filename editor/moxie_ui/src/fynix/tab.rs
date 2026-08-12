//! A leaf's tab bar: the strip, the scrolling row, and one tab.

use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::window::SystemCursorIcon;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

use super::{ButtonElem, Icon, Label};
use crate::widgets::dock::{DockTab, DockTabRow, TAB_HEIGHT, TabId};

/// The strip across the top of a leaf.
#[derive(Element, OverrideDefault, Lenz)]
pub struct TabBar {
    #[default(Color::srgba(1.0, 1.0, 1.0, 0.03))]
    pub background: Color,
}

impl ElementVisual<BevyHost> for TabBar {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert((
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
        world: &mut World,
        node: Entity,
        field: TabBarField,
    ) {
        match field {
            TabBarField::Background => {
                world
                    .entity_mut(node)
                    .insert(BackgroundColor(self.background));
            }
        }
    }
}

/// What the tabs themselves sit in, which scrolls when they overflow.
#[derive(Element, OverrideDefault, Lenz)]
pub struct TabRow;

impl ElementVisual<BevyHost> for TabRow {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert((
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
        _world: &mut World,
        _node: Entity,
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
#[derive(Element, OverrideDefault, Lenz)]
pub struct Tab {
    #[elem]
    pub icon: Option<Icon>,
    #[elem]
    pub label: Label,
    #[elem]
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
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert((
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
        world: &mut World,
        node: Entity,
        field: TabField,
    ) {
        let mut entity = world.entity_mut(node);

        match field {
            TabField::Active | TabField::Fill => {
                entity.insert(self.background());
            }
            TabField::WindowId | TabField::Tab => {
                entity.insert(DockTab {
                    window_id: self.window_id.clone(),
                    tab_id: self.tab,
                });
            }
        }
    }
}
