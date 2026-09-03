use crate::reactive::{FynixBuild, FynixHost};
use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::Button as ButtonBehavior;
use bevy::window::SystemCursorIcon;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;
use fynix::style::Style;

use super::patch::*;
use super::{Icon, IconCursor, Label, LabelCursor};
use crate::motion::LitFrom as _;
use crate::theme::EditorTheme;

/// What lights up under the cursor, and to what colour. A style has
/// no node to wire this on, so it leaves the choice here for the
/// `#[element(build = ...)]` hook to read once the node exists.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum Hover {
    /// Nothing lights up.
    #[default]
    None,
    /// The surface itself.
    Fill(Color),
    /// The icon and label, not the surface.
    IconLabel(Color),
}

/// A hit area holding an icon, a label, both, or whatever is built
/// under it. Rests at the theme's fill, sized for a toolbar, and
/// lights up under the cursor.
#[element(build = Self::build)]
pub struct Button {
    /// A node of its own, so its image and colour can be bound
    /// without touching the button.
    #[elem(child)]
    pub icon: Option<Icon>,
    /// A node of its own, so its text can be bound without touching
    /// the button.
    #[elem(child)]
    pub label: Option<Label>,
    /// Between the icon and the label, when both are there.
    #[elem(default = px(theme.space.md), patch = PatchColumnGap)]
    pub column_gap: Val,
    /// The surface at rest. [`Color::NONE`] for a button that sits on
    /// something already a surface.
    #[elem(default = theme.color.fill, patch = PatchBackground)]
    pub fill: Color,
    #[elem(default = px(theme.space.touch), patch = PatchWidth)]
    pub width: Val,
    #[elem(default = px(theme.space.touch), patch = PatchHeight)]
    pub height: Val,
    /// Share of a flex row's remaining space this button claims, for
    /// one that should fill a row rather than size to its own content.
    #[elem(patch = PatchFlexGrow)]
    pub flex_grow: f32,
    #[elem(default = px(18), patch = PatchMinWidth)]
    pub min_width: Val,
    #[elem(default = px(18), patch = PatchMinHeight)]
    pub min_height: Val,
    /// Centred, for a button that is only as big as what it holds.
    #[elem(default = ::Center, patch = PatchJustify)]
    pub justify: JustifyContent,
    #[elem(patch = PatchPadding)]
    pub padding: UiRect,
    #[elem(default = px(theme.space.md), patch = PatchRadius)]
    pub radius: Val,
    /// Overrides `radius` with independent corners, for a button that
    /// sits at one end of a row of others (a
    /// [`SegmentedControl`](super::SegmentedControl)'s outer
    /// segments). `None` rounds all four corners by `radius`.
    #[elem(patch = PatchCorners)]
    pub corners: Option<BorderRadius>,
    /// What lights up under the cursor. See [`Hover`].
    #[elem(ignore, default = Hover::Fill(theme.color.hover))]
    hover: Hover,
}

impl Button {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            Node {
                align_items: AlignItems::Center,
                ..default()
            },
            ButtonBehavior,
            EntityCursor::System(SystemCursorIcon::Pointer),
        ));

        // Wired against a base already in hand as `self`: the node
        // has no entry in the kernel's table yet for `.lit()` to read
        // one from. Both stops light to the same colour: `Hover`
        // carries one shade, not separate hover/press ones.
        let (fill, icon_label) = match self.hover {
            Hover::None => (None, None),
            Hover::Fill(color) => (Some(color), None),
            Hover::IconLabel(color) => (None, Some(color)),
        };

        if let Some(color) = fill {
            build.lit_from(
                |button| button.fill(),
                self.fill,
                color,
                color,
            );
        }

        if let Some(tint) = icon_label {
            // Read straight from `self`: an absent icon/label is
            // skipped, same as if it were never lit at all.
            if let Some(icon) = &self.icon {
                build.lit_from(
                    |button| button.icon().color(),
                    icon.color,
                    tint,
                    tint,
                );
            }
            // `Color::NONE` is the themed default, no base to lerp
            // from.
            if let Some(color) = self
                .label
                .as_ref()
                .map(|label| label.color)
                .filter(|color| *color != Color::NONE)
            {
                build.lit_from(
                    |button| button.label().color(),
                    color,
                    tint,
                    tint,
                );
            }
        }
    }
}

/// A button whose icon and label carry `tint` under the cursor - the
/// accent, by default, for the one action in a group the eye should
/// land on first. No surface of its own.
#[derive(Default)]
pub struct TintButton {
    /// `None` takes the theme's own accent.
    pub tint: Option<Color>,
}

impl Style for TintButton {
    type Host = FynixHost;
    type Element = Button;

    fn apply(self, button: &mut Button, theme: &EditorTheme) {
        button.fill = Color::NONE;
        button.width = Val::Auto;
        button.height = Val::Auto;
        button.radius = Val::ZERO;
        button.hover =
            Hover::IconLabel(self.tint.unwrap_or(theme.color.accent));
    }
}

/// A menu bar's own button: square, full height, lighting up only
/// under the cursor, so a row of them reads as one strip.
pub struct MenuButton;

impl Style for MenuButton {
    type Host = FynixHost;
    type Element = Button;

    fn apply(self, button: &mut Button, _theme: &EditorTheme) {
        button.fill = Color::NONE;
        button.width = Val::Auto;
        button.height = percent(100);
        button.radius = Val::ZERO;
        button.padding = UiRect::axes(px(10), Val::ZERO);
    }
}

/// One segment of a [`SegmentedControl`](super::SegmentedControl):
/// filled solid with the theme's accent when `active`, its theme fill
/// otherwise. Rounding is the control's own, not each segment's - it
/// clips its row of children rather than rounding them individually.
pub struct SegmentButton {
    pub active: bool,
}

impl Style for SegmentButton {
    type Host = FynixHost;
    type Element = Button;

    fn apply(self, button: &mut Button, theme: &EditorTheme) {
        button.width = Val::Auto;
        button.height = px(theme.space.row);
        button.radius = Val::ZERO;
        button.flex_grow = 1.0;
        button.fill = if self.active {
            theme.color.accent
        } else {
            theme.color.fill
        };
        button.hover = if self.active {
            Hover::None
        } else {
            Hover::Fill(theme.color.hover)
        };
    }
}

/// A button with no surface of its own until the cursor is on it, for
/// one that sits in a row of its own kind or on something that is
/// already a surface.
pub struct GhostButton;

impl Style for GhostButton {
    type Host = FynixHost;
    type Element = Button;

    fn apply(self, button: &mut Button, theme: &EditorTheme) {
        button.fill = Color::NONE;
        button.width = Val::Auto;
        button.height = Val::Auto;
        button.padding =
            UiRect::axes(px(theme.space.lg), px(theme.space.sm));
        button.radius = px(theme.space.radius);
    }
}

// Overrides `radius` when set; leaves it be when `None`, so the two
// can be patched independently.
field_patch!(PatchCorners, Option<BorderRadius>, |patch, v| {
    if let Some(radius) = *v {
        node(patch, move |n| n.border_radius = radius);
    }
});
