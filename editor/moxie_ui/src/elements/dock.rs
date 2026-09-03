//! What the reconciler builds a [`DockTree`] out of.
//!
//! One element per node the tree materializes into: the host it all
//! hangs from, a split and its handle, the panels either side, an
//! area and the content of one of its tabs.
//!
//! [`DockTree`]: crate::widgets::dock::DockTree

use crate::reactive::FynixBuild;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::patch::*;

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
    #[elem(default = NodeId(0), patch = PatchGroupNode)]
    pub node: NodeId,
    #[elem(default = 0.05, patch = PatchGroupMinRatio)]
    pub min_ratio: f32,
    #[elem(default = ::Row, patch = PatchGroupAxis)]
    pub axis: FlexDirection,
}

field_patch!(PatchGroupNode, NodeId, |patch, v| {
    patch.insert(NodeBinding(*v));
});

field_patch!(PatchGroupMinRatio, f32, |patch, v| {
    patch.insert(PanelGroup { min_ratio: *v });
});

field_patch!(PatchGroupAxis, FlexDirection, |patch, v| {
    patch.insert(filled(*v));
});

/// What a split is dragged by: a full-sized hit area holding a
/// slim, always-visible [`line`](Self::line) and a wider
/// [`bar`](Self::bar) that only shows on hover.
#[element(build = Self::build)]
pub struct SplitHandle {
    #[elem(default = NodeId(0), patch = PatchHandleNode)]
    pub node: NodeId,
    #[elem(default = ::Row, patch = PatchHandleAxis)]
    pub axis: FlexDirection,
    /// Hidden when either side of the split has collapsed: there is
    /// nothing left to drag between.
    #[elem(default = true, patch = PatchHandleVisible)]
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
            // A small hit area centring `bar`; `axis` pulls it back
            // onto the seam with a negative margin, `visible` toggles
            // `display`.
            Node {
                min_width: px(HANDLE_SIZE),
                min_height: px(HANDLE_SIZE),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            // Overlaps both panels by design; without it the second one,
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

field_patch!(PatchHandleNode, NodeId, |patch, v| {
    patch.insert(NodeBinding(*v));
});

field_patch!(PatchHandleAxis, FlexDirection, |patch, v| {
    let margin = handle_margin(*v);
    node(patch, move |n| n.margin = margin);
});

field_patch!(PatchHandleVisible, bool, |patch, v| {
    let display = display(*v);
    node(patch, move |n| n.display = display);
});

/// One side of a split. The ratio is bound rather than built:
/// dragging the handle rewrites it every frame, and must not rebuild
/// what the panel holds.
#[element(build = Self::build)]
pub struct SplitPanel {
    #[elem(default = 1.0, patch = PatchPanelRatio)]
    pub ratio: f32,
    /// A collapsed panel takes no space, so its sibling reclaims it.
    #[elem(default = true, patch = PatchPanelVisible)]
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

field_patch!(PatchPanelRatio, f32, |patch, v| {
    patch.insert(Panel { ratio: *v });
});

field_patch!(PatchPanelVisible, bool, |patch, v| {
    let size = if *v { percent(100) } else { px(0) };
    let display = display(*v);
    node(patch, move |n| {
        n.width = size;
        n.height = size;
        n.display = display;
    });
});

/// A leaf of the tree: a tab bar, and the content of every tab.
#[element(build = Self::build)]
pub struct Area {
    #[elem(default = NodeId(0), patch = PatchAreaNode)]
    pub node: NodeId,
    #[elem(patch = PatchAreaId)]
    pub id: String,
    #[elem(default = ::TabBar, patch = PatchAreaStyle)]
    pub style: DockAreaStyle,
    /// Which tab is showing, which a binding keeps up to date. The
    /// component itself, because a walk hops *into* an `Option`
    /// rather than naming it, and `None` is a value this has to
    /// carry.
    #[elem(patch = PatchAreaActive)]
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

field_patch!(PatchAreaActive, ActiveDockWindow, |patch, v| {
    patch.insert(v.clone());
});

field_patch!(PatchAreaNode, NodeId, |patch, v| {
    patch.insert(NodeBinding(*v));
});

field_patch!(PatchAreaId, String, |patch, v| {
    with::<DockArea>(patch, |area| area.id.clone_from(v));
});

field_patch!(PatchAreaStyle, DockAreaStyle, |patch, v| {
    with::<DockArea>(patch, |area| area.style = v.clone());
});

/// One tab's content. Switching tabs flips `display` through a
/// binding rather than rebuilding: the content owns cameras, scroll
/// offsets and live edits that have to survive a tab switch.
#[element(build = Self::build)]
pub struct TabContent {
    #[elem(patch = PatchContentWindowId)]
    pub window_id: String,
    #[elem(default = TabId(0), patch = PatchContentTab)]
    pub tab: TabId,
    #[elem(patch = PatchContentShowing)]
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

// Only `display`: the content pane's own layout is its business, and
// writing the whole `Node` would clobber whatever it did to itself.
field_patch!(PatchContentShowing, bool, |patch, v| {
    let display = display(*v);
    node(patch, move |n| n.display = display);
});

field_patch!(PatchContentWindowId, String, |patch, v| {
    with::<DockWindow>(patch, |w| w.descriptor_id.clone_from(v));
    with::<DockTabContent>(patch, |c| c.window_id.clone_from(v));
});

field_patch!(PatchContentTab, TabId, |patch, v| {
    with::<DockWindow>(patch, |w| w.tab_id = *v);
    with::<DockTabContent>(patch, |c| c.tab_id = *v);
});
