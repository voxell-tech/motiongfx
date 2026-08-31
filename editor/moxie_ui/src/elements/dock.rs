//! What the reconciler builds a [`DockTree`] out of.
//!
//! One element per node the tree materializes into: the host it all
//! hangs from, a split and its handle, the panels either side, an
//! area and the content of one of its tabs.
//!
//! [`DockTree`]: crate::widgets::dock::DockTree

use crate::reactive::{FynixBuild, FynixHost};
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;
use fynix::ui::Patch;

use super::patch::{node, with};

use super::Frame;
use crate::widgets::dock::{
    ActiveDockWindow, DockArea, DockAreaStyle, DockTabContent,
    DockTreeHost, DockWindow, HANDLE_SIZE, NodeBinding, NodeId,
    Panel, PanelGroup, PanelHandle, TabId,
};

/// Column that fills its parent, which most of the dock's nodes are.
///
/// `min_width`/`min_height` at `0`, not `Node`'s own default `Auto`,
/// which floors a node at its content's size regardless of the `100%`
/// above.
pub(super) fn filled(direction: FlexDirection) -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        min_width: px(0),
        min_height: px(0),
        flex_direction: direction,
        overflow: Overflow::clip(),
        ..default()
    }
}

pub(super) fn display(visible: bool) -> Display {
    if visible {
        Display::Flex
    } else {
        Display::None
    }
}

/// The node the whole tree is rendered underneath.
#[element(build = Self::build)]
pub struct DockHost;

impl DockHost {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            DockTreeHost,
            Node {
                width: percent(100),
                height: percent(100),
                min_width: px(0),
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ));
    }
}

/// A split: two panels and the handle between them. Each field
/// writes its own component whole, so it needs no build hook.
#[element]
pub struct SplitGroup {
    #[elem(patch = patch_group_node)]
    #[default(NodeId(0))]
    pub node: NodeId,
    #[elem(patch = patch_group_min_ratio)]
    #[default(0.05)]
    pub min_ratio: f32,
    #[elem(patch = patch_group_axis)]
    #[default(::Row)]
    pub axis: FlexDirection,
}

fn patch_group_node(patch: &mut Patch<FynixHost>, node: &NodeId) {
    patch.insert(NodeBinding(*node));
}

fn patch_group_min_ratio(
    patch: &mut Patch<FynixHost>,
    min_ratio: &f32,
) {
    patch.insert(PanelGroup {
        min_ratio: *min_ratio,
    });
}

fn patch_group_axis(
    patch: &mut Patch<FynixHost>,
    axis: &FlexDirection,
) {
    patch.insert(filled(*axis));
}

/// What a split is dragged by: a full-sized hit area holding a
/// slim, always-visible [`line`](Self::line) and a wider
/// [`bar`](Self::bar) that only shows on hover.
#[element(build = Self::build)]
pub struct SplitHandle {
    #[elem(patch = patch_handle_node)]
    #[default(NodeId(0))]
    pub node: NodeId,
    #[elem(patch = patch_handle_axis)]
    #[default(::Row)]
    pub axis: FlexDirection,
    /// Hidden when either side of the split has collapsed: there is
    /// nothing left to drag between.
    #[elem(patch = patch_handle_visible)]
    #[default(true)]
    pub visible: bool,
    /// Marks the seam at rest. Never interactive, and never lit -
    /// [`handle_line`] gives it a fixed color.
    #[elem(child)]
    pub line: Frame,
    /// Half the hit area and centred in it, so the seam reads flush
    /// until the cursor finds it. `lit` on this is what actually
    /// colors the handle.
    #[elem(child)]
    pub bar: Frame,
}

/// The bar's own size: thin on the split's axis, full-length across
/// it.
pub fn handle_bar(axis: FlexDirection) -> Frame {
    let thin = px(HANDLE_SIZE / 2.0);
    match axis {
        FlexDirection::Row | FlexDirection::RowReverse => Frame {
            width: thin,
            height: percent(100),
            ..default()
        },
        FlexDirection::Column | FlexDirection::ColumnReverse => {
            Frame {
                width: percent(100),
                height: thin,
                ..default()
            }
        }
    }
}

/// One pixel, centered in the hit area by explicit inset rather than
/// flex alignment - it sits outside the flow so `bar` can still
/// center itself normally.
pub fn handle_line(axis: FlexDirection, color: Color) -> Frame {
    const LINE: f32 = 1.0;
    let offset = px((HANDLE_SIZE - LINE) / 2.0);
    let base = Frame {
        position: PositionType::Absolute,
        background: color,
        ..default()
    };

    match axis {
        FlexDirection::Row | FlexDirection::RowReverse => Frame {
            width: px(LINE),
            height: percent(100),
            inset: UiRect::horizontal(offset),
            ..base
        },
        FlexDirection::Column | FlexDirection::ColumnReverse => {
            Frame {
                width: percent(100),
                height: px(LINE),
                inset: UiRect::vertical(offset),
                ..base
            }
        }
    }
}

impl SplitHandle {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            PanelHandle,
            NodeBinding(self.node),
            self.node(),
            // Overlaps both panels by design; without self the second one,
            // painted after it, would win the pointer.
            ZIndex(1),
        ));
    }
}

/// The negative pull that lands the hit area back on the seam, on the
/// axis the split runs along.
pub(super) fn handle_margin(axis: FlexDirection) -> UiRect {
    let pull = px(-HANDLE_SIZE / 2.0);
    match axis {
        FlexDirection::Row | FlexDirection::RowReverse => {
            UiRect::horizontal(pull)
        }
        FlexDirection::Column | FlexDirection::ColumnReverse => {
            UiRect::vertical(pull)
        }
    }
}

fn patch_handle_node(patch: &mut Patch<FynixHost>, node: &NodeId) {
    patch.insert(NodeBinding(*node));
}

fn patch_handle_axis(
    patch: &mut Patch<FynixHost>,
    axis: &FlexDirection,
) {
    let margin = handle_margin(*axis);
    node(patch, move |n| n.margin = margin);
}

fn patch_handle_visible(
    patch: &mut Patch<FynixHost>,
    visible: &bool,
) {
    let display = display(*visible);
    node(patch, move |n| n.display = display);
}

/// One side of a split. The ratio is bound rather than built:
/// dragging the handle rewrites it every frame, and must not rebuild
/// what the panel holds.
#[element(build = Self::build)]
pub struct SplitPanel {
    #[elem(patch = patch_panel_ratio)]
    #[default(1.0)]
    pub ratio: f32,
    /// A collapsed panel takes no space, so its sibling reclaims it.
    #[elem(patch = patch_panel_visible)]
    #[default(true)]
    pub visible: bool,
}

impl SplitPanel {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert(Node {
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip(),
            ..default()
        });
    }
}

fn patch_panel_ratio(patch: &mut Patch<FynixHost>, ratio: &f32) {
    patch.insert(Panel { ratio: *ratio });
}

fn patch_panel_visible(patch: &mut Patch<FynixHost>, visible: &bool) {
    let size = if *visible { percent(100) } else { px(0) };
    let display = display(*visible);
    node(patch, move |n| {
        n.width = size;
        n.height = size;
        n.display = display;
    });
}

/// A leaf of the tree: a tab bar, and the content of every tab.
#[element(build = Self::build)]
pub struct Area {
    #[elem(patch = patch_area_node)]
    #[default(NodeId(0))]
    pub node: NodeId,
    #[elem(patch = patch_area_id)]
    pub id: String,
    #[elem(patch = patch_area_style)]
    #[default(::TabBar)]
    pub style: DockAreaStyle,
    /// Which tab is showing, which a binding keeps up to date. The
    /// component itself, because a walk hops *into* an `Option`
    /// rather than naming it, and `None` is a value this has to
    /// carry.
    #[elem(patch = patch_area_active)]
    pub active: ActiveDockWindow,
}

impl Area {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            DockArea {
                id: self.id.clone(),
                style: self.style.clone(),
            },
            self.active.clone(),
            NodeBinding(self.node),
            filled(FlexDirection::Column),
        ));
    }
}

fn patch_area_active(
    patch: &mut Patch<FynixHost>,
    active: &ActiveDockWindow,
) {
    patch.insert(active.clone());
}

fn patch_area_node(patch: &mut Patch<FynixHost>, node: &NodeId) {
    patch.insert(NodeBinding(*node));
}

fn patch_area_id(patch: &mut Patch<FynixHost>, id: &String) {
    with::<DockArea>(patch, |area| area.id.clone_from(id));
}

fn patch_area_style(
    patch: &mut Patch<FynixHost>,
    style: &DockAreaStyle,
) {
    with::<DockArea>(patch, |area| area.style = style.clone());
}

/// One tab's content. Switching tabs flips `display` through a
/// binding rather than rebuilding: the content owns cameras, scroll
/// offsets and live edits that have to survive a tab switch.
#[element(build = Self::build)]
pub struct TabContent {
    #[elem(patch = patch_content_window_id)]
    pub window_id: String,
    #[elem(patch = patch_content_tab)]
    #[default(TabId(0))]
    pub tab: TabId,
    #[elem(patch = patch_content_showing)]
    pub showing: bool,
}

impl TabContent {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            DockWindow {
                descriptor_id: self.window_id.clone(),
                tab_id: self.tab,
            },
            DockTabContent {
                window_id: self.window_id.clone(),
                tab_id: self.tab,
            },
            Node {
                flex_grow: 1.0,
                width: percent(100),
                min_width: px(0),
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                display: display(self.showing),
                ..default()
            },
        ));
    }
}

/// Only `display`: the content pane's own layout is its business, and
/// writing the whole `Node` would clobber whatever it did to itself.
fn patch_content_showing(
    patch: &mut Patch<FynixHost>,
    showing: &bool,
) {
    let display = display(*showing);
    node(patch, move |n| n.display = display);
}

fn patch_content_window_id(
    patch: &mut Patch<FynixHost>,
    window_id: &String,
) {
    with::<DockWindow>(patch, |w| {
        w.descriptor_id.clone_from(window_id)
    });
    with::<DockTabContent>(patch, |c| {
        c.window_id.clone_from(window_id)
    });
}

fn patch_content_tab(patch: &mut Patch<FynixHost>, tab: &TabId) {
    with::<DockWindow>(patch, |w| w.tab_id = *tab);
    with::<DockTabContent>(patch, |c| c.tab_id = *tab);
}
