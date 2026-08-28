//! What the reconciler builds a [`DockTree`] out of.
//!
//! One element per node the tree materializes into: the host it all
//! hangs from, a split and its handle, the panels either side, an
//! area and the content of one of its tabs.
//!
//! [`DockTree`]: crate::widgets::dock::DockTree

use crate::reactive::BevyHost;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::{Element, ElementVisual};
use fynix::ui::{Build, Patch};

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
fn filled(direction: FlexDirection) -> Node {
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

fn display(visible: bool) -> Display {
    if visible {
        Display::Flex
    } else {
        Display::None
    }
}

/// The node the whole tree is rendered underneath.
#[derive(Element)]
pub struct DockHost;

impl ElementVisual<BevyHost> for DockHost {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
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

    fn patch_fields(
        &self,
        _patch: &mut Patch<BevyHost>,
        field: DockHostField,
    ) {
        match field {}
    }
}

/// A split: two panels and the handle between them.
#[derive(Element)]
pub struct SplitGroup {
    #[default(NodeId(0))]
    pub node: NodeId,
    #[default(0.05)]
    pub min_ratio: f32,
    #[default(::Row)]
    pub axis: FlexDirection,
}

impl ElementVisual<BevyHost> for SplitGroup {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((
            NodeBinding(self.node),
            PanelGroup {
                min_ratio: self.min_ratio,
            },
            filled(self.axis),
        ));
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: SplitGroupField,
    ) {
        match field {
            SplitGroupField::Node => {
                patch.insert(NodeBinding(self.node));
            }
            SplitGroupField::MinRatio => {
                patch.insert(PanelGroup {
                    min_ratio: self.min_ratio,
                });
            }
            SplitGroupField::Axis => {
                patch.insert(filled(self.axis));
            }
        }
    }
}

/// What a split is dragged by: a full-sized hit area holding a
/// slim, always-visible [`line`](Self::line) and a wider
/// [`bar`](Self::bar) that only shows on hover.
#[derive(Element)]
pub struct SplitHandle {
    #[default(NodeId(0))]
    pub node: NodeId,
    #[default(::Row)]
    pub axis: FlexDirection,
    /// Hidden when either side of the split has collapsed: there is
    /// nothing left to drag between.
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

impl SplitHandle {
    /// Full-sized hit area, pulled back onto the seam by a matching
    /// negative margin so the panels read as touching. Centers
    /// [`bar`](Self::bar), which carries its own thinner size.
    fn node(&self) -> Node {
        let pull = px(-HANDLE_SIZE / 2.0);
        let margin = match self.axis {
            FlexDirection::Row | FlexDirection::RowReverse => {
                UiRect::horizontal(pull)
            }
            FlexDirection::Column | FlexDirection::ColumnReverse => {
                UiRect::vertical(pull)
            }
        };

        Node {
            min_width: px(HANDLE_SIZE),
            min_height: px(HANDLE_SIZE),
            margin,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            display: display(self.visible),
            ..default()
        }
    }
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

impl ElementVisual<BevyHost> for SplitHandle {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((
            PanelHandle,
            NodeBinding(self.node),
            self.node(),
            // Overlaps both panels by design; without this the
            // second one, painted after it, would win the pointer.
            ZIndex(1),
        ));
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: SplitHandleField,
    ) {
        match field {
            SplitHandleField::Node => {
                patch.insert(NodeBinding(self.node));
            }
            SplitHandleField::Axis | SplitHandleField::Visible => {
                patch.insert(self.node());
            }
        }
    }
}

/// One side of a split. The ratio is bound rather than built:
/// dragging the handle rewrites it every frame, and must not rebuild
/// what the panel holds.
#[derive(Element)]
pub struct SplitPanel {
    #[default(1.0)]
    pub ratio: f32,
    /// A collapsed panel takes no space, so its sibling reclaims it.
    #[default(true)]
    pub visible: bool,
}

impl SplitPanel {
    fn node(&self) -> Node {
        let size = if self.visible { percent(100) } else { px(0) };

        Node {
            width: size,
            height: size,
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip(),
            display: display(self.visible),
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for SplitPanel {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((Panel { ratio: self.ratio }, self.node()));
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: SplitPanelField,
    ) {
        match field {
            SplitPanelField::Ratio => {
                patch.insert(Panel { ratio: self.ratio });
            }
            SplitPanelField::Visible => {
                patch.insert(self.node());
            }
        }
    }
}

/// A leaf of the tree: a tab bar, and the content of every tab.
#[derive(Element)]
pub struct Area {
    #[default(NodeId(0))]
    pub node: NodeId,
    pub id: String,
    #[default(::TabBar)]
    pub style: DockAreaStyle,
    /// Which tab is showing, which a binding keeps up to date. The
    /// component itself, because a walk hops *into* an `Option`
    /// rather than naming it, and `None` is a value this has to
    /// carry.
    pub active: ActiveDockWindow,
}

impl ElementVisual<BevyHost> for Area {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
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

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: AreaField,
    ) {
        match field {
            AreaField::Active => {
                patch.insert(self.active.clone());
            }
            AreaField::Node => {
                patch.insert(NodeBinding(self.node));
            }
            AreaField::Id | AreaField::Style => {
                patch.insert(DockArea {
                    id: self.id.clone(),
                    style: self.style.clone(),
                });
            }
        }
    }
}

/// One tab's content. Switching tabs flips `display` through a
/// binding rather than rebuilding: the content owns cameras, scroll
/// offsets and live edits that have to survive a tab switch.
#[derive(Element)]
pub struct TabContent {
    pub window_id: String,
    #[default(TabId(0))]
    pub tab: TabId,
    pub showing: bool,
}

impl ElementVisual<BevyHost> for TabContent {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
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

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: TabContentField,
    ) {
        match field {
            // Only `display`: the content pane's own layout is its
            // business, and writing the whole `Node` would clobber
            // whatever it did to itself.
            TabContentField::Showing => {
                if let Some(mut layout) =
                    patch.entity_mut().get_mut::<Node>()
                {
                    layout.display = display(self.showing);
                }
            }
            TabContentField::WindowId | TabContentField::Tab => {
                patch.insert((
                    DockWindow {
                        descriptor_id: self.window_id.clone(),
                        tab_id: self.tab,
                    },
                    DockTabContent {
                        window_id: self.window_id.clone(),
                        tab_id: self.tab,
                    },
                ));
            }
        }
    }
}
