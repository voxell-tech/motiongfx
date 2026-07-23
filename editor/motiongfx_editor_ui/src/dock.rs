//! Generic docking/panel system: a pure-data [`DockTree`] of splits
//! and tabbed leaves, a reconciler that materializes it into UI,
//! resizable splits, tab bars, and `Pointer<Drag>`-driven drag/drop
//! (reorder tabs, merge into another leaf, or split an area on drop).

mod add_popup;
mod area;
mod drag;
mod reconcile;
mod registry;
mod split;
mod tabs;
mod tree;

pub use area::{
    ActiveDockWindow, DockArea, DockTab, DockTabAddButton,
    DockTabBar, DockTabCloseButton, DockTabContent, DockWindow,
};
use bevy::prelude::*;
pub use drag::{DockDragPlugin, DockDragState};
pub use reconcile::{
    DockTreeHost, NodeBinding, ReconcilePlugin, dock,
};
pub use registry::{
    DockWindowBuildFn, DockWindowDescriptor, WindowRegistry,
};
pub use split::{
    Panel, PanelGroup, PanelHandle, SplitPanelPlugin, panel,
    panel_group, panel_handle,
};
pub use tabs::DockTabRow;
pub use tree::{
    DockAreaStyle, DockLeaf, DockNode, DockSplit, DockTabEntry,
    DockTree, Edge, NodeId, SplitAxis, TabId,
};

/// Assembles the docking engine: resizable splits, tab bars, the tree
/// reconciler, and drag/drop. Does not seed a [`DockTree`] or
/// register any [`WindowRegistry`] entries; callers do that after
/// adding this plugin.
pub struct DockPlugin;

impl Plugin for DockPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            super::glass::GlassPlugin,
            split::SplitPanelPlugin,
            drag::DockDragPlugin,
            reconcile::ReconcilePlugin,
        ))
        .init_resource::<registry::WindowRegistry>();
    }
}

// Backgrounds come from the glass materials (`crate::glass`) and
// colors from the theme (`crate::theme`); only metrics live here.

/// Tab-bar height, in px.
pub(crate) const TAB_HEIGHT: f32 = 32.0;
