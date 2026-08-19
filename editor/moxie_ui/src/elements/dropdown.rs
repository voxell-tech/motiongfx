//! A dropdown, built on the menu `bevy_ui_widgets` already ships.
//!
//! Three elements rather than one, because that is the shape the
//! behaviour expects: an anchor holding a control and a list, which
//! is how the menu's own observer finds each of them. What comes with
//! it is everything a hand-rolled popup gets wrong - placement that
//! flips near the window edge, dismissal on focus loss, Escape, and
//! arrow-key navigation.

use bevy::feathers::controls::{FeathersMenu, FeathersMenuPopup};
use bevy::feathers::cursor::EntityCursor;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::scene::EntityWorldMutSceneExt;
use bevy::ui_widgets::{
    ActivateOnPress, Button as ButtonBehavior, MenuButton, MenuItem,
};
use bevy::window::SystemCursorIcon;
use bevy_fynix::BevyUi;
use bevy_fynix::host::BevyHost;
use fynix_mock::element::{Element, ElementVisual};

use super::{Icon, Label};

/// What a [`Dropdown`] and its [`DropdownList`] hang from.
///
/// Carries the observer that opens and closes the list, which reaches
/// both by looking through this node's children - so the two must be
/// built underneath it, in either order.
#[derive(Element)]
pub struct DropdownMenu;

impl ElementVisual<BevyHost> for DropdownMenu {
    fn build_fields(&self, ui: &mut BevyUi<'_>) {
        let node = ui.parent();
        let world = &mut *ui.world;

        if let Err(err) =
            world.entity_mut(node).apply_scene(bsn! { @FeathersMenu })
        {
            error!("failed to build a dropdown: {err}");
        }
    }

    fn patch_fields(
        &self,
        _world: &mut World,
        _node: Entity,
        field: DropdownMenuField,
    ) {
        match field {}
    }
}

/// The shut control: what is chosen now, and a chevron.
#[derive(Element)]
pub struct Dropdown {
    /// A node of its own, so the choice showing can be bound without
    /// the control being rebuilt.
    #[elem(child)]
    pub label: Label,
    #[elem(child)]
    pub chevron: Icon,
    /// Wide enough to stay readable when the choice is a short word.
    #[default(px(72))]
    pub min_width: Val,
    /// Past this the label is clipped rather than the row growing:
    /// a column of these has to line up.
    #[default(px(160))]
    pub max_width: Val,
    #[default(px(22))]
    pub height: Val,
    #[default(Color::srgba(1.0, 1.0, 1.0, 0.06))]
    pub fill: Color,
    #[default(px(4))]
    pub radius: Val,
}

impl Dropdown {
    /// A width that fits the longest of `options`.
    ///
    /// Approximate: text is not measured until after layout has run,
    /// so this works from the character count at `font_size` and
    /// leaves room for the chevron. It errs wide, because the point
    /// is a control that does not resize as the choice changes.
    pub fn width_for(options: &[String], font_size: f32) -> Val {
        /// Rough advance width of one character, as a share of the
        /// font's size.
        const ADVANCE: f32 = 0.62;
        /// Padding either side, the chevron, and the gap before it.
        const FURNITURE: f32 = 8.0 + 8.0 + 9.0 + 6.0;

        let longest = options
            .iter()
            .map(|option| option.chars().count())
            .max()
            .unwrap_or_default();

        px(longest as f32 * font_size * ADVANCE + FURNITURE)
    }

    fn node(&self) -> Node {
        Node {
            min_width: self.min_width,
            max_width: self.max_width,
            height: self.height,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(6),
            padding: UiRect::axes(px(8), Val::ZERO),
            border_radius: BorderRadius::all(self.radius),
            // What will not fit is the label's problem, not the
            // row's - see `hold_chevron`.
            overflow: Overflow::clip(),
            ..default()
        }
    }

    /// Keeps the chevron its own size while the label gives way.
    fn hold_chevron(ui: &mut BevyUi<'_>) {
        let Some(chevron) = ui.child(DropdownCursor::chevron) else {
            return;
        };

        if let Some(mut layout) = ui.world.get_mut::<Node>(chevron) {
            layout.flex_shrink = 0.0;
        }
    }
}

impl ElementVisual<BevyHost> for Dropdown {
    fn build_fields(&self, ui: &mut BevyUi<'_>) {
        let node = ui.parent();

        ui.world.entity_mut(node).insert((
            self.node(),
            BackgroundColor(self.fill),
            ButtonBehavior,
            // Opens the list beside it; the anchor's own observer
            // does the work.
            ActivateOnPress,
            MenuButton,
            EntityCursor::System(SystemCursorIcon::Pointer),
        ));
        Self::hold_chevron(ui);
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: DropdownField,
    ) {
        let mut entity = world.entity_mut(node);

        match field {
            DropdownField::Fill => {
                entity.insert(BackgroundColor(self.fill));
            }
            DropdownField::MinWidth
            | DropdownField::MaxWidth
            | DropdownField::Height
            | DropdownField::Radius => {
                entity.insert(self.node());
            }
        }
    }
}

/// The list, which shows for as long as it holds focus.
///
/// Placed by the popup scene it is built from, so it flips above the
/// control rather than off the bottom of the window, and is not
/// clipped by whatever it was opened inside.
#[derive(Element)]
pub struct DropdownList {
    /// Matched to the control's, so the two line up.
    #[default(px(160))]
    pub width: Val,
    /// What the popup scene rounds its own corners to, so leaving this
    /// alone keeps the look feathers gave it.
    #[default(px(4))]
    pub radius: Val,
}

impl DropdownList {
    /// Only the width and the corners. The rest of the node belongs to
    /// the popup scene, and writing it whole would undo the placement
    /// that came with it.
    fn size(&self, world: &mut World, node: Entity) {
        if let Some(mut layout) = world.get_mut::<Node>(node) {
            layout.min_width = self.width;
            layout.border_radius = BorderRadius::all(self.radius);
        }
    }
}

impl ElementVisual<BevyHost> for DropdownList {
    fn build_fields(&self, ui: &mut BevyUi<'_>) {
        let node = ui.parent();
        let world = &mut *ui.world;

        if let Err(err) = world
            .entity_mut(node)
            .apply_scene(bsn! { @FeathersMenuPopup })
        {
            error!("failed to build a dropdown list: {err}");
            return;
        }
        self.size(world, node);
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: DropdownListField,
    ) {
        match field {
            DropdownListField::Width | DropdownListField::Radius => {
                self.size(world, node)
            }
        }
    }
}

/// One row of a [`DropdownList`].
///
/// Picking one closes the list. It also has to be focusable, because
/// focus is what keeps the list open at all.
#[derive(Element)]
pub struct DropdownItem {
    #[elem(child)]
    pub label: Label,
    #[default(px(20))]
    pub height: Val,
    #[default(::NONE)]
    pub fill: Color,
    #[default(px(3))]
    pub radius: Val,
}

impl DropdownItem {
    fn node(&self) -> Node {
        Node {
            width: percent(100),
            height: self.height,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(8), Val::ZERO),
            border_radius: BorderRadius::all(self.radius),
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for DropdownItem {
    fn build_fields(&self, ui: &mut BevyUi<'_>) {
        let node = ui.parent();
        let world = &mut *ui.world;

        world.entity_mut(node).insert((
            self.node(),
            BackgroundColor(self.fill),
            MenuItem,
            TabIndex(0),
            EntityCursor::System(SystemCursorIcon::Pointer),
        ));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: DropdownItemField,
    ) {
        let mut entity = world.entity_mut(node);

        match field {
            DropdownItemField::Fill => {
                entity.insert(BackgroundColor(self.fill));
            }
            DropdownItemField::Height | DropdownItemField::Radius => {
                entity.insert(self.node());
            }
        }
    }
}
