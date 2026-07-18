//! A generic docking/panel system: a pure-data [`DockTree`]
//! describing splits and tabbed leaves, a reconciler that
//! materializes it into UI entities, resizable splits, tab bars, and
//! `Pointer<Drag>`-driven drag/drop for reordering tabs, merging them
//! into another leaf, or splitting an area on drop.
//!
//! Ported from a sibling project's generic docking engine
//! (`jackdaw_panels`). Not yet wired into
//! the editor app or the timeline panel — register
//! [`DockPlugin`] explicitly, register window kinds on
//! [`WindowRegistry`], spawn a [`DockTreeHost`] entity, and seed a
//! [`DockTree`].
//!
//! Deliberately out of scope for this first port: an icon-only
//! sidebar area style, multi-workspace save/switch, and an "add
//! window" popup menu — just splits, tabs, and drag/drop.

mod add_popup;
mod area;
mod drag;
mod reconcile;
mod registry;
mod split;
mod tabs;
mod tree;

pub use area::{
    ActiveDockWindow, DockArea, DockTab, DockTabBar,
    DockTabCloseButton, DockTabContent, DockWindow,
};
use bevy::prelude::*;
pub use drag::{DockDragPlugin, DockDragState};
pub use reconcile::{DockTreeHost, NodeBinding, ReconcilePlugin};
pub use registry::{
    DockWindowBuildFn, DockWindowDescriptor, WindowRegistry,
};
pub use split::{
    Panel, PanelGroup, PanelHandle, SplitPanelPlugin, panel,
    panel_group, panel_handle,
};
pub use tabs::{DockTabAddButton, DockTabPlugin, DockTabRow};
pub use tree::{
    DockAreaStyle, DockLeaf, DockNode, DockSplit, DockTabEntry,
    DockTree, Edge, NodeId, SplitAxis, TabId,
};

/// Assembles the docking engine: resizable splits, tab bars, the tree
/// reconciler, and drag/drop. Does not seed a [`DockTree`] or
/// register any [`WindowRegistry`] entries — callers do that after
/// adding this plugin.
pub struct DockPlugin;

impl Plugin for DockPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            super::glass::GlassPlugin,
            split::SplitPanelPlugin,
            tabs::DockTabPlugin,
            drag::DockDragPlugin,
            reconcile::ReconcilePlugin,
            add_popup::AddWindowPopupPlugin,
        ))
        .init_resource::<registry::WindowRegistry>();
    }
}

// Backgrounds come from the glass materials (`crate::glass`) and
// colors from the theme (`crate::theme`); only metrics live here.

/// Tab-bar height, in px.
pub(crate) const TAB_HEIGHT: f32 = 32.0;
