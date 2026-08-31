//! A dropdown, built on the menu `bevy_ui_widgets` already ships.
//!
//! Three elements, the shape the behaviour expects: an anchor
//! holding a control and a list, which is how the menu's own
//! observer finds each of them. In exchange it gets everything a
//! hand-rolled popup gets wrong: placement that flips near the
//! window edge, dismissal on focus loss, Escape, and arrow-key
//! navigation.

use crate::reactive::{FynixBuild, FynixHost};
use bevy::feathers::controls::{FeathersMenu, FeathersMenuPopup};
use bevy::feathers::cursor::EntityCursor;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::scene::EntityWorldMutSceneExt;
use bevy::ui_widgets::{
    ActivateOnPress, Button as ButtonBehavior, MenuButton, MenuItem,
};
use bevy::window::SystemCursorIcon;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;
use fynix::ui::Patch;

use super::patch::{self, node};
use super::{Icon, Label};

/// What a [`Dropdown`] and its [`DropdownList`] hang from.
///
/// Carries the observer that opens and closes the list, found by
/// looking through this node's children. Both must be built
/// underneath it, in either order.
#[element(build = Self::build)]
pub struct DropdownMenu;

impl DropdownMenu {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        if let Err(err) =
            build.entity_mut().apply_scene(bsn! { @FeathersMenu })
        {
            error!("failed to build a dropdown: {err}");
        }
    }
}

/// The shut control: what is chosen now, and a chevron.
#[element(build = Self::build)]
pub struct Dropdown {
    /// A node of its own, so the choice showing can be bound without
    /// the control being rebuilt.
    #[elem(child)]
    pub label: Label,
    #[elem(child)]
    pub chevron: Icon,
    /// Wide enough to stay readable when the choice is a short word.
    #[elem(patch = patch::min_width)]
    #[default(px(72))]
    pub min_width: Val,
    /// Past this the label is clipped rather than the row growing:
    /// a column of these has to line up.
    #[elem(patch = patch::max_width)]
    #[default(px(160))]
    pub max_width: Val,
    #[elem(patch = patch::height)]
    #[default(px(22))]
    pub height: Val,
    #[elem(patch = patch::background)]
    #[default(Color::srgba(1.0, 1.0, 1.0, 0.06))]
    pub fill: Color,
    #[elem(patch = patch::radius)]
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

    /// Keeps the chevron its own size while the label gives way.
    fn hold_chevron(build: &mut FynixBuild<'_, Self>) {
        let Some(chevron) = build.child(DropdownCursor::chevron)
        else {
            return;
        };

        if let Some(mut layout) = build.world.get_mut::<Node>(chevron)
        {
            layout.flex_shrink = 0.0;
        }
    }
}

impl Dropdown {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px(6),
                padding: UiRect::axes(px(8), Val::ZERO),
                // What will not fit is the label's problem, not the
                // row's - see `hold_chevron`.
                overflow: Overflow::clip(),
                ..default()
            },
            ButtonBehavior,
            // Opens the list beside it; the anchor's own observer does
            // the work.
            ActivateOnPress,
            MenuButton,
            EntityCursor::System(SystemCursorIcon::Pointer),
        ));
        Dropdown::hold_chevron(build);
    }
}

/// The list, which shows for as long as it holds focus.
///
/// Placed by the popup scene it is built from, so it flips above the
/// control rather than off the bottom of the window, and is not
/// clipped by whatever it was opened inside.
#[element(build = Self::build)]
pub struct DropdownList {
    /// Matched to the control's, so the two line up.
    #[elem(patch = list_width)]
    #[default(px(160))]
    pub width: Val,
    /// What the popup scene rounds its own corners to, so leaving this
    /// alone keeps the look feathers gave it.
    #[elem(patch = patch::radius)]
    #[default(px(4))]
    pub radius: Val,
}

impl DropdownList {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        if let Err(err) = build
            .entity_mut()
            .apply_scene(bsn! { @FeathersMenuPopup })
        {
            error!("failed to build a dropdown list: {err}");
            return;
        }
        // Only the width and the corners: the rest of the node belongs to
        // the popup scene, and writing it whole would undo the placement.
        if let Some(mut layout) = build.entity_mut().get_mut::<Node>()
        {
            layout.min_width = self.width;
            layout.border_radius = BorderRadius::all(self.radius);
        }
    }
}

/// The list is placed by its popup scene, so only its own width lands
/// on `min_width`.
fn list_width(patch: &mut Patch<FynixHost>, width: &Val) {
    node(patch, |n| n.min_width = *width);
}

/// One row of a [`DropdownList`].
///
/// Picking one closes the list. It also has to be focusable, because
/// focus is what keeps the list open at all.
#[element(build = Self::build)]
pub struct DropdownItem {
    #[elem(child)]
    pub label: Label,
    #[elem(patch = patch::height)]
    #[default(px(20))]
    pub height: Val,
    #[elem(patch = patch::background)]
    #[default(::NONE)]
    pub fill: Color,
    #[elem(patch = patch::radius)]
    #[default(px(3))]
    pub radius: Val,
}

impl DropdownItem {}

impl DropdownItem {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(8), Val::ZERO),
                ..default()
            },
            MenuItem,
            TabIndex(0),
            EntityCursor::System(SystemCursorIcon::Pointer),
        ));
    }
}
