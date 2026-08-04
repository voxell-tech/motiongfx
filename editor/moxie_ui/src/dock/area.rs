//! Component markers for dock areas, tabs, and their content.

use bevy::feathers::constants::icons;
use bevy::feathers::cursor::EntityCursor;
use bevy::picking::events::{Click, Pointer};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::widget::ImageNode;
use bevy::window::SystemCursorIcon;

use super::reconcile::LeafBinding;
use super::tabs::{DockTabCloseIcon, tab_tile_node};
use super::tree::{DockTree, TabId};
use crate::glass::Glass;

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

/// Props for [`DockTab`]. `area` is the leaf area entity, captured by
/// the click observer so activating a tab needs no parent walk.
#[derive(Clone)]
pub struct DockTabProps {
    pub window_id: String,
    pub tab_id: TabId,
    pub label: String,
    pub area: Entity,
    pub is_active: bool,
    pub text_color: Color,
    pub close_color: Color,
}

impl Default for DockTabProps {
    fn default() -> Self {
        Self {
            window_id: String::new(),
            tab_id: TabId::default(),
            label: String::new(),
            area: Entity::PLACEHOLDER,
            is_active: false,
            text_color: Color::WHITE,
            close_color: Color::NONE,
        }
    }
}

#[derive(SceneComponent, Default, Clone)]
#[scene(DockTabProps)]
pub struct DockTab {
    pub window_id: String,
    pub tab_id: TabId,
}

impl DockTab {
    fn scene(props: DockTabProps) -> impl Scene {
        let DockTabProps {
            window_id,
            tab_id,
            label,
            area,
            is_active,
            text_color,
            close_color,
        } = props;
        // `@area` has no meaningful default: omitting it silently
        // yields a placeholder and the tab never activates.
        debug_assert_ne!(
            area,
            Entity::PLACEHOLDER,
            "DockTab requires `@area`"
        );
        let close_id = window_id.clone();
        bsn! {
            DockTab {
                window_id: {window_id},
                tab_id: {tab_id},
            }
            // Drives the hover pill + close-icon fade.
            Hovered
            on(super::tabs::on_tab_hover)
            // Active pill vs faint idle; swapped by `sync_leaf_visuals`.
            template_value(Glass::tab(is_active))
            // Tabs are draggable: signal it on hover.
            EntityCursor::System(SystemCursorIcon::Grab)
            on(move |mut click: On<Pointer<Click>>,
                     bindings: Query<&LeafBinding>,
                     mut tree: ResMut<DockTree>| {
                click.propagate(false);
                if let Ok(binding) = bindings.get(area) {
                    tree.set_active(binding.0, tab_id);
                }
            })
            template_value(tab_tile_node())
            Children [
                (
                    Text({label})
                    TextLayout { linebreak: LineBreak::NoWrap }
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        weight: FontWeight::BOLD,
                    }
                    TextColor({text_color})
                ),
                // Slot always reserves its 14x14 so the tab doesn't
                // reflow on hover; the icon is alpha-toggled.
                @DockTabCloseButton {
                    @window_id: {close_id},
                    @tab_id: {tab_id},
                    @icon_color: {close_color},
                }
            ]
        }
    }
}

/// Props for [`DockTabCloseButton`]. `icon_color` is the resting
/// color; its alpha is driven by `show_close_on_hover`.
#[derive(Default, Clone)]
pub struct DockTabCloseButtonProps {
    pub window_id: String,
    pub tab_id: TabId,
    pub icon_color: Color,
}

#[derive(SceneComponent, Default, Clone)]
#[scene(DockTabCloseButtonProps)]
pub struct DockTabCloseButton {
    pub window_id: String,
    pub tab_id: TabId,
}

impl DockTabCloseButton {
    fn scene(props: DockTabCloseButtonProps) -> impl Scene {
        let DockTabCloseButtonProps {
            window_id,
            tab_id,
            icon_color,
        } = props;
        bsn! {
            DockTabCloseButton {
                window_id: {window_id},
                tab_id: {tab_id},
            }
            EntityCursor::System(SystemCursorIcon::Pointer)
            on(move |mut click: On<Pointer<Click>>,
                     mut tree: ResMut<DockTree>| {
                click.propagate(false);
                // Keep at least one tab alive across the layout.
                if tree.tabs().count() > 1 {
                    tree.remove_tab(tab_id);
                }
            })
            Node {
                width: Val::Px(14.0),
                height: Val::Px(14.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(2.0)),
            }
            Children [(
                DockTabCloseIcon
                ImageNode {
                    image: {icons::X},
                    color: {icon_color},
                }
                Node { width: Val::Px(10.0), height: Val::Px(10.0) }
            )]
        }
    }
}

/// Props for [`DockTabAddButton`]. `area` is the leaf area entity the
/// popup adds tabs to.
#[derive(Clone)]
pub struct DockTabAddButtonProps {
    pub area: Entity,
    pub icon_color: Color,
}

impl Default for DockTabAddButtonProps {
    fn default() -> Self {
        Self {
            area: Entity::PLACEHOLDER,
            icon_color: Color::WHITE,
        }
    }
}

#[derive(SceneComponent, Clone)]
#[scene(DockTabAddButtonProps)]
pub struct DockTabAddButton {
    pub area_entity: Entity,
}

/// `Entity` has no `Default`, so the get-or-insert the derive relies
/// on needs one written by hand.
impl Default for DockTabAddButton {
    fn default() -> Self {
        Self {
            area_entity: Entity::PLACEHOLDER,
        }
    }
}

impl DockTabAddButton {
    fn scene(props: DockTabAddButtonProps) -> impl Scene {
        let DockTabAddButtonProps { area, icon_color } = props;
        // `@area` has no meaningful default: omitting it silently
        // yields a placeholder and the popup adds to nothing.
        debug_assert_ne!(
            area,
            Entity::PLACEHOLDER,
            "DockTabAddButton requires `@area`"
        );
        bsn! {
            DockTabAddButton { area_entity: {area} }
            EntityCursor::System(SystemCursorIcon::Pointer)
            on(super::add_popup::on_add_click)
            Node {
                width: Val::Px(18.0),
                height: Val::Px(18.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
            }
            Children [(
                Text("+")
                TextFont { font_size: FontSize::Px(11.0) }
                TextColor({icon_color})
            )]
        }
    }
}

#[derive(Component)]
pub struct DockTabContent {
    pub window_id: String,
    pub tab_id: TabId,
}
