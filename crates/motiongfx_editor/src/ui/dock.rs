//! A generic docking/panel system: a pure-data [`DockTree`] describing
//! splits and tabbed leaves, a reconciler that materializes it into UI
//! entities, resizable splits, tab bars, and `Pointer<Drag>`-driven
//! drag/drop for reordering tabs, merging them into another leaf, or
//! splitting an area on drop.
//!
//! Ported from a sibling project's generic docking engine
//! (`jackdaw_panels`). Not yet wired into [`crate::MotionGfxEditorPlugin`]
//! or the timeline panel — register [`DockPlugin`] explicitly, register
//! window kinds on [`WindowRegistry`], spawn a [`DockTreeHost`] entity,
//! and seed a [`DockTree`].
//!
//! Deliberately out of scope for this first port: an icon-only sidebar
//! area style, multi-workspace save/switch, and an "add window" popup
//! menu — just splits, tabs, and drag/drop.

mod add_popup;
mod area;
mod drag;
mod reconcile;
mod registry;
mod split;
mod tabs;
mod tree;

pub use area::{ActiveDockWindow, DockArea, DockTab, DockTabBar, DockTabCloseButton, DockTabContent, DockWindow};
pub use drag::{DockDragPlugin, DockDragState};
pub use reconcile::{DockTreeHost, NodeBinding, ReconcilePlugin};
pub use registry::{DockWindowBuildFn, DockWindowDescriptor, WindowRegistry};
pub use split::{Panel, PanelGroup, PanelHandle, SplitPanelPlugin, panel, panel_group, panel_handle};
pub use tabs::{DockTabAddButton, DockTabPlugin, DockTabRow};
pub use tree::{
    DockAreaStyle, DockLeaf, DockNode, DockSplit, DockTabEntry, DockTree, Edge, NodeId, SplitAxis,
    TabId,
};

use bevy::prelude::*;

// Flat color palette for the dock UI. `bevy_feathers` tokens are
// `ThemeToken`s resolved through the theme system, but these spawn
// helpers set raw `BackgroundColor`/`TextColor`/`BorderColor`, so the
// module keeps its own dark-theme constants (matching the flat-palette
// approach of the engine this was ported from).

/// Tab-bar height, in px.
pub(crate) const TAB_HEIGHT: f32 = 32.0;

/// Tab-bar background.
pub(crate) const PANEL_HEADER_BG: Color = Color::srgb(0.13, 0.13, 0.15);
/// Tab-bar / panel border.
pub(crate) const PANEL_BORDER: Color = Color::srgb(0.25, 0.25, 0.28);
/// Primary (active) text color.
pub(crate) const TEXT_MAIN: Color = Color::srgb(0.92, 0.92, 0.94);
/// Floating menu / drag-ghost background.
pub(crate) const MENU_BG: Color = Color::srgb(0.16, 0.16, 0.19);
/// Active-tab background.
pub(crate) const TAB_ACTIVE_BG: Color = Color::srgba(1.0, 1.0, 1.0, 0.06);
/// General accent color for drag ghosts and drop-target borders; also
/// the active-tab top border.
pub(crate) const ACCENT_COLOR: Color = Color::srgb(0.35, 0.55, 0.95);
pub(crate) const TAB_ACTIVE_BORDER: Color = ACCENT_COLOR;
/// Inactive-tab text/icon color.
pub(crate) const TAB_INACTIVE_TEXT: Color = Color::srgba(1.0, 1.0, 1.0, 0.55);
/// Base tint for drag drop-target overlays (alpha applied at each use site).
pub(crate) const DROP_OVERLAY_BASE: Color = ACCENT_COLOR;

/// Assembles the docking engine: resizable splits, tab bars, the tree
/// reconciler, and drag/drop. Does not seed a [`DockTree`] or register
/// any [`WindowRegistry`] entries — callers do that after adding this
/// plugin.
pub struct DockPlugin;

impl Plugin for DockPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            split::SplitPanelPlugin,
            tabs::DockTabPlugin,
            drag::DockDragPlugin,
            reconcile::ReconcilePlugin,
            add_popup::AddWindowPopupPlugin,
        ))
        .init_resource::<registry::WindowRegistry>();
    }
}
