//! Component markers for dock areas, tabs, and their content.

use bevy::prelude::*;

use super::tree::TabId;

#[derive(Component, Clone, Debug)]
pub struct DockArea {
    pub id: String,
    pub style: super::tree::DockAreaStyle,
}

#[derive(Component, Clone, Debug)]
pub struct DockWindow {
    pub descriptor_id: String,
    /// Per-instance handle. Two `DockWindow` entities with the same
    /// `descriptor_id` (e.g. two Outliner tabs) carry distinct
    /// `tab_id`s so the reconciler / activate / close paths can tell
    /// them apart.
    pub tab_id: TabId,
}

/// `Some(tab_id)` of the active tab in this leaf, or `None` for an
/// empty leaf. Reconciler reads this to decide which content entity
/// to show. Tracking by `TabId` rather than `window_id` lets two tabs
/// of the same window kind coexist without their content stacking.
#[derive(Component, Clone, Debug, Default)]
pub struct ActiveDockWindow(pub Option<TabId>);

#[derive(Component)]
pub struct DockTabBar;

#[derive(Component)]
pub struct DockTab {
    pub window_id: String,
    pub tab_id: TabId,
}

#[derive(Component)]
pub struct DockTabCloseButton {
    pub window_id: String,
    pub tab_id: TabId,
}

#[derive(Component)]
pub struct DockTabContent {
    pub window_id: String,
    pub tab_id: TabId,
}
