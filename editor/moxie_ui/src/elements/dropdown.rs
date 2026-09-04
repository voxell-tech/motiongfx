//! A dropdown, built on the menu `bevy_ui_widgets` already ships.
//!
//! Three elements, the shape the behaviour expects: an anchor
//! holding a control and a list, which is how the menu's own
//! observer finds each of them. In exchange it gets everything a
//! hand-rolled popup gets wrong: placement that flips near the
//! window edge, dismissal on focus loss, Escape, and arrow-key
//! navigation.

use crate::reactive::FynixBuild;
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
use crate::motion::Lit;
use bevy_fynix::tag::{Hovered, Pressed};
use fynix::element::element;

use super::patch::*;
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
    #[elem(default = px(72), patch = PatchMinWidth)]
    pub min_width: Val,
    /// Past this the label is clipped rather than the row growing:
    /// a column of these has to line up.
    #[elem(default = px(160), patch = PatchMaxWidth)]
    pub max_width: Val,
    #[elem(default = px(22), patch = PatchHeight)]
    pub height: Val,
    #[elem(default = theme.color.fill, patch = PatchBackground, anim(
        duration = theme.motion.interact,
        ease = theme.motion.ease,
        lerp = <Color as Lit>::mix,
        on(Pressed, read = Self::pressed),
        on(Hovered, read = Self::hovered),
    ))]
    pub fill: Color,
    /// What `fill` travels to under the cursor. [`Color::NONE`]
    /// leaves it at rest.
    #[elem(ignore, default = ::NONE)]
    pub hover_fill: Color,
    /// While held. Falls back to `hover_fill` when unset.
    #[elem(ignore, default = ::NONE)]
    pub press_fill: Color,
    #[elem(default = px(4), patch = PatchRadius)]
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
    #[elem(default = px(160), patch = PatchListWidth)]
    pub width: Val,
    /// What the popup scene rounds its own corners to, so leaving this
    /// alone keeps the look feathers gave it.
    #[elem(default = px(4), patch = PatchRadius)]
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
        // The rest of the node belongs to the popup scene, and writing
        // it whole would undo the placement. Its vertical padding is
        // zeroed so the rows sit flush.
        if let Some(mut layout) = build.entity_mut().get_mut::<Node>()
        {
            layout.min_width = self.width;
            layout.border_radius = BorderRadius::all(self.radius);
            layout.padding.top = px(0);
            layout.padding.bottom = px(0);
        }
    }
}

// The list is placed by its popup scene, so only its own width lands
// on `min_width`.
field_patch!(PatchListWidth, Val, |patch, v| {
    node(patch, |n| n.min_width = *v);
});

/// One row of a [`DropdownList`].
///
/// Picking one closes the list. It also has to be focusable, because
/// focus is what keeps the list open at all.
#[element(build = Self::build)]
pub struct DropdownItem {
    #[elem(child)]
    pub label: Label,
    #[elem(default = px(20), patch = PatchHeight)]
    pub height: Val,
    #[elem(default = ::NONE, patch = PatchBackground, anim(
        duration = theme.motion.interact,
        ease = theme.motion.ease,
        lerp = <Color as Lit>::mix,
        on(Pressed, read = Self::pressed),
        on(Hovered, read = Self::hovered),
    ))]
    pub fill: Color,
    /// What `fill` travels to under the cursor. [`Color::NONE`]
    /// leaves it at rest.
    #[elem(ignore, default = ::NONE)]
    pub hover_fill: Color,
    /// While held. Falls back to `hover_fill` when unset.
    #[elem(ignore, default = ::NONE)]
    pub press_fill: Color,
    #[elem(default = px(3), patch = PatchRadius)]
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

impl Dropdown {
    /// Where `fill` heads under the cursor, or its own colour when
    /// none was set, so it stays put.
    fn hovered(&self) -> &Color {
        if self.hover_fill == Color::NONE {
            &self.fill
        } else {
            &self.hover_fill
        }
    }

    /// While held, falling back to the hover shade.
    fn pressed(&self) -> &Color {
        if self.press_fill == Color::NONE {
            self.hovered()
        } else {
            &self.press_fill
        }
    }
}

impl DropdownItem {
    /// Where `fill` heads under the cursor, or its own colour when
    /// none was set, so it stays put.
    fn hovered(&self) -> &Color {
        if self.hover_fill == Color::NONE {
            &self.fill
        } else {
            &self.hover_fill
        }
    }

    /// While held, falling back to the hover shade.
    fn pressed(&self) -> &Color {
        if self.press_fill == Color::NONE {
            self.hovered()
        } else {
            &self.press_fill
        }
    }
}
