//! What the reconciler builds a [`DockTree`] out of.
//!
//! One element per node the tree materializes into: the host it all
//! hangs from, a split and its handle, the panels either side, an
//! area and the content of one of its tabs.
//!
//! [`DockTree`]: crate::widgets::dock::DockTree

use bevy::prelude::*;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

use crate::widgets::dock::{
    ActiveDockWindow, DockArea, DockAreaStyle, DockTabContent,
    DockTreeHost, DockWindow, HANDLE_SIZE, NodeBinding, NodeId,
    Panel, PanelGroup, PanelHandle, TabId,
};

/// Column that fills its parent, which most of the dock's nodes are.
fn filled(direction: FlexDirection) -> Node {
    Node {
        width: percent(100),
        height: percent(100),
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
#[derive(Element, OverrideDefault, Lenz)]
pub struct DockHost;

impl ElementVisual<BevyHost> for DockHost {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert((
            DockTreeHost,
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ));
    }

    fn patch_fields(
        &self,
        _world: &mut World,
        _node: Entity,
        field: DockHostField,
    ) {
        match field {}
    }
}

/// A split: two panels and the handle between them.
#[derive(Element, OverrideDefault, Lenz)]
pub struct SplitGroup {
    #[default(NodeId(0))]
    pub node: NodeId,
    #[default(0.05)]
    pub min_ratio: f32,
    #[default(::Row)]
    pub axis: FlexDirection,
}

impl ElementVisual<BevyHost> for SplitGroup {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert((
            NodeBinding(self.node),
            PanelGroup {
                min_ratio: self.min_ratio,
            },
            filled(self.axis),
        ));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: SplitGroupField,
    ) {
        let mut entity = world.entity_mut(node);

        match field {
            SplitGroupField::Node => {
                entity.insert(NodeBinding(self.node));
            }
            SplitGroupField::MinRatio => {
                entity.insert(PanelGroup {
                    min_ratio: self.min_ratio,
                });
            }
            SplitGroupField::Axis => {
                entity.insert(filled(self.axis));
            }
        }
    }
}

/// What a split is dragged by.
#[derive(Element, OverrideDefault, Lenz)]
pub struct SplitHandle {
    #[default(NodeId(0))]
    pub node: NodeId,
    #[default(::Row)]
    pub axis: FlexDirection,
    /// Hidden when either side of the split has collapsed: there is
    /// nothing left to drag between.
    #[default(true)]
    pub visible: bool,
}

impl SplitHandle {
    /// Full-sized hit area, pulled back onto the seam by a matching
    /// negative margin so the panels read as touching.
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
            display: display(self.visible),
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for SplitHandle {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert((
            PanelHandle,
            NodeBinding(self.node),
            self.node(),
            BackgroundColor(Color::NONE),
        ));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: SplitHandleField,
    ) {
        let mut entity = world.entity_mut(node);

        match field {
            SplitHandleField::Node => {
                entity.insert(NodeBinding(self.node));
            }
            SplitHandleField::Axis | SplitHandleField::Visible => {
                entity.insert(self.node());
            }
        }
    }
}

/// One side of a split. The ratio is bound rather than built:
/// dragging the handle rewrites it every frame, and must not rebuild
/// what the panel holds.
#[derive(Element, OverrideDefault, Lenz)]
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
    fn build_fields(&self, world: &mut World, node: Entity) {
        world
            .entity_mut(node)
            .insert((Panel { ratio: self.ratio }, self.node()));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: SplitPanelField,
    ) {
        let mut entity = world.entity_mut(node);

        match field {
            SplitPanelField::Ratio => {
                entity.insert(Panel { ratio: self.ratio });
            }
            SplitPanelField::Visible => {
                entity.insert(self.node());
            }
        }
    }
}

/// A leaf of the tree: a tab bar, and the content of every tab.
#[derive(Element, OverrideDefault, Lenz)]
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
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert((
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
        world: &mut World,
        node: Entity,
        field: AreaField,
    ) {
        let mut entity = world.entity_mut(node);

        match field {
            AreaField::Active => {
                entity.insert(self.active.clone());
            }
            AreaField::Node => {
                entity.insert(NodeBinding(self.node));
            }
            AreaField::Id | AreaField::Style => {
                entity.insert(DockArea {
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
#[derive(Element, OverrideDefault, Lenz)]
pub struct TabContent {
    pub window_id: String,
    #[default(TabId(0))]
    pub tab: TabId,
    pub showing: bool,
}

impl ElementVisual<BevyHost> for TabContent {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert((
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
        world: &mut World,
        node: Entity,
        field: TabContentField,
    ) {
        match field {
            // Only `display`: the content pane's own layout is its
            // business, and writing the whole `Node` would clobber
            // whatever it did to itself.
            TabContentField::Showing => {
                if let Some(mut layout) = world.get_mut::<Node>(node)
                {
                    layout.display = display(self.showing);
                }
            }
            TabContentField::WindowId | TabContentField::Tab => {
                world.entity_mut(node).insert((
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
