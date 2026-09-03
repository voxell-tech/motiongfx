//! A leaf's tab bar: the strip, the scrolling row, and one tab.

use crate::reactive::FynixBuild;
use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::window::SystemCursorIcon;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;

use super::patch::*;
use super::{ButtonElem, Icon, Label};
use crate::widgets::dock::{DockTab, DockTabRow, TAB_HEIGHT, TabId};

/// The strip across the top of a leaf.
#[element(build = Self::build)]
pub struct TabBar {
    #[elem(patch = PatchBackground)]
    #[default(Color::srgba(1.0, 1.0, 1.0, 0.03))]
    pub background: Color,
}

impl TabBar {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                width: percent(100),
                height: px(TAB_HEIGHT),
                // No left padding: the first tab sits flush.
                padding: UiRect::new(
                    Val::ZERO,
                    px(8),
                    px(1),
                    Val::ZERO,
                ),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(self.background),
        ));
    }
}

/// What the tabs themselves sit in, which scrolls when they overflow.
#[element(build = Self::build)]
pub struct TabRow;

impl TabRow {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            DockTabRow,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(2),
                height: percent(100),
                overflow: Overflow::scroll_x(),
                flex_shrink: 1.0,
                min_width: px(0),
                ..default()
            },
            ScrollPosition::default(),
        ));
    }
}

/// One tab: its icon, its name, and the button that closes it.
///
/// Which tab is active is a field rather than a rebuild, because
/// switching tabs must not take the drag in progress or the row's
/// scroll offset with it.
#[element(build = Self::build)]
pub struct Tab {
    #[elem(child)]
    pub icon: Option<Icon>,
    #[elem(child)]
    pub label: Label,
    #[elem(child)]
    pub close: Option<ButtonElem>,
    #[elem(patch = PatchWindowId)]
    pub window_id: String,
    #[elem(patch = PatchTabId)]
    #[default(TabId(0))]
    pub tab: TabId,
    #[elem(patch = PatchActive)]
    pub active: bool,
    #[elem(patch = PatchTabFill)]
    #[default(Color::srgba(1.0, 1.0, 1.0, 0.06))]
    pub fill: Color,
}

/// The tab's fill, kept on the node whether or not it is showing, so
/// [`PatchActive`] can put it back when the tab becomes active again.
#[derive(Component, Clone, Copy)]
pub(super) struct TabFill(pub(super) Color);

/// Marks the tab as active. Kept on the node so [`PatchTabFill`] knows
/// whether to show a new fill without reading it back off
/// [`BackgroundColor`], which an active tab with a [`Color::NONE`] fill
/// reports as inactive.
#[derive(Component)]
pub(super) struct TabActive;

pub(super) fn tab_background(
    active: bool,
    fill: Color,
) -> BackgroundColor {
    BackgroundColor(if active { fill } else { Color::NONE })
}

impl Tab {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            DockTab {
                window_id: self.window_id.clone(),
                tab_id: self.tab,
            },
            // A tab is dragged, so it says so at rest. The drag itself
            // swaps in `Grabbing` through `OverrideCursor`.
            EntityCursor::System(SystemCursorIcon::Grab),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                padding: UiRect::axes(px(8), px(3)),
                height: percent(100),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            TabFill(self.fill),
            tab_background(self.active, self.fill),
        ));
        if self.active {
            build.insert(TabActive);
        }
    }
}

field_patch!(PatchActive, bool, |patch, v| {
    let fill = patch
        .entity_mut()
        .get::<TabFill>()
        .map(|f| f.0)
        .unwrap_or(Color::NONE);
    patch.insert(tab_background(*v, fill));
    if *v {
        patch.insert(TabActive);
    } else {
        patch.remove::<TabActive>();
    }
});

field_patch!(PatchTabFill, Color, |patch, v| {
    let active = patch.entity_mut().contains::<TabActive>();
    patch.insert(TabFill(*v));
    if active {
        patch.insert(BackgroundColor(*v));
    }
});

field_patch!(PatchWindowId, String, |patch, v| {
    with::<DockTab>(patch, |d| d.window_id.clone_from(v));
});

field_patch!(PatchTabId, TabId, |patch, v| {
    with::<DockTab>(patch, |d| d.tab_id = *v);
});

#[cfg(test)]
mod tests {
    use fynix::ui::{FieldPatch, Patch};

    use super::*;
    use crate::reactive::FynixHost;
    use crate::theme::EditorTheme;

    /// An active tab whose fill is [`Color::NONE`] still picks up a
    /// later non-transparent fill: [`PatchTabFill`] reads [`TabActive`],
    /// not the (transparent) [`BackgroundColor`] left behind by
    /// activation.
    #[test]
    fn fill_updates_active_tab_with_none_fill() {
        let mut world = World::new();
        let theme = EditorTheme::default();

        let node = world
            .spawn((
                TabFill(Color::NONE),
                tab_background(true, Color::NONE),
            ))
            .id();

        let green = Color::srgb(0.0, 1.0, 0.0);

        {
            let mut patch =
                Patch::<FynixHost>::new(&mut world, node, &theme);
            PatchActive::patch(&mut patch, &true);
        }
        {
            let mut patch =
                Patch::<FynixHost>::new(&mut world, node, &theme);
            PatchTabFill::patch(&mut patch, &green);
        }

        assert_eq!(
            world.get::<BackgroundColor>(node).unwrap().0,
            green
        );
        assert_eq!(world.get::<TabFill>(node).unwrap().0, green);
    }
}
