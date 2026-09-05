//! What a dock area, its tabs, and their content are marked with.

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
    /// Per instance handle. Two `DockWindow` entities of the same
    /// `descriptor_id` (two Outliner tabs, say) carry distinct
    /// `tab_id`s, so the reconcile, activate and close paths can tell
    /// them apart.
    pub tab_id: TabId,
}

/// `Some(tab_id)` of the active tab in this leaf, or `None` for an
/// empty one. Tracked by [`TabId`] rather than window id, so two tabs
/// of the same kind coexist without their content stacking.
#[derive(Component, Clone, Debug, Default)]
pub struct ActiveDockWindow(pub Option<TabId>);

/// One tab in a bar.
#[derive(Component, Clone, Debug, Default)]
pub struct DockTab {
    pub window_id: String,
    pub tab_id: TabId,
}

/// The "+" at the end of a bar, and the area its popup adds to.
#[derive(Component, Clone, Debug)]
pub struct DockTabAddButton {
    pub area_entity: Entity,
}

/// A tab's content pane.
#[derive(Component)]
pub struct DockTabContent {
    pub window_id: String,
    pub tab_id: TabId,
}
