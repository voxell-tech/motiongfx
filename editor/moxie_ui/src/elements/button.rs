use crate::reactive::{FynixBuild, FynixHost};
use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::Button as ButtonBehavior;
use bevy::window::SystemCursorIcon;
use bevy_fynix::WorldEntityMut as _;
use fynix::element::element;
use fynix::style::Style;
use fynix::ui::Patch;

use super::patch::{self, node};
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
    /// [`Button`], [`GhostButton`], [`MenuButton`]: the surface
    /// itself.
    Fill(Color),
    /// [`TintButton`]: the icon and label, not the surface.
    IconLabel(Color),
}

/// A hit area holding an icon, a label, both, or whatever is built
/// under it, with no look of its own. [`Button`] and [`GhostButton`]
/// are two the editor gives it.
#[element(build = Self::build)]
pub struct ButtonElem {
    /// A node of its own, so its image and colour can be bound
    /// without touching the button.
    #[elem(child)]
    pub icon: Option<Icon>,
    /// A node of its own, so its text can be bound without touching
    /// the button.
    #[elem(child)]
    pub label: Option<Label>,
    /// Between the icon and the label, when both are there.
    #[default(px(6))]
    #[elem(patch = patch::column_gap)]
    pub column_gap: Val,
    /// What the background shows. Nothing by default, which is a
    /// [`GhostButton`]; [`Button`] rests at the theme's own fill, and
    /// interaction lights either of them up.
    #[default(::NONE)]
    #[elem(patch = patch::background)]
    pub fill: Color,
    #[elem(patch = patch::width)]
    pub width: Val,
    #[elem(patch = patch::height)]
    pub height: Val,
    /// Share of a flex row's remaining space this button claims, for
    /// one that should fill a row rather than size to its own content.
    #[elem(patch = patch::flex_grow)]
    pub flex_grow: f32,
    #[default(px(18))]
    #[elem(patch = patch::min_width)]
    pub min_width: Val,
    #[default(px(18))]
    #[elem(patch = patch::min_height)]
    pub min_height: Val,
    /// Centred, for a button that is only as big as what it holds.
    #[default(::Center)]
    #[elem(patch = patch::justify)]
    pub justify: JustifyContent,
    #[elem(patch = patch::padding)]
    pub padding: UiRect,
    #[default(::ZERO)]
    #[elem(patch = patch::radius)]
    pub radius: Val,
    /// Overrides `radius` with independent corners, for a button that
    /// sits at one end of a row of others (a
    /// [`SegmentedControl`](super::SegmentedControl)'s outer
    /// segments). `None` rounds all four corners by `radius`.
    #[elem(patch = corners)]
    pub corners: Option<BorderRadius>,
    /// Set by whichever [`Style`] built this - see [`Hover`]. Never
    /// patched: read once, when the lanes are wired.
    #[elem(ignore)]
    hover: Hover,
}

/// The editor's own button: a filled, rounded pill sized for a
/// toolbar, that lights up under the cursor.
pub struct Button;

impl Style for Button {
    type Host = FynixHost;
    type Element = ButtonElem;

    fn apply(self, button: &mut ButtonElem, theme: &EditorTheme) {
        button.fill = theme.button_fill;
        button.width = px(26);
        button.height = px(26);
        button.radius = px(6);
        button.hover = Hover::Fill(theme.hover_overlay);
    }
}

/// A button whose icon and label carry `tint` under the cursor - the
/// accent, by default, for the one action in a group the eye should
/// land on first.
#[derive(Default)]
pub struct TintButton {
    /// `None` takes the theme's own accent.
    pub tint: Option<Color>,
}

impl Style for TintButton {
    type Host = FynixHost;
    type Element = ButtonElem;

    fn apply(self, button: &mut ButtonElem, theme: &EditorTheme) {
        button.hover =
            Hover::IconLabel(self.tint.unwrap_or(theme.accent));
    }
}

/// A menu bar's own button: square, full height, lighting up only
/// under the cursor, so a row of them reads as one strip.
pub struct MenuButton;

impl Style for MenuButton {
    type Host = FynixHost;
    type Element = ButtonElem;

    fn apply(self, button: &mut ButtonElem, theme: &EditorTheme) {
        button.fill = Color::NONE;
        button.radius = Val::ZERO;
        button.height = percent(100);
        button.padding = UiRect::axes(px(10), Val::ZERO);
        button.hover = Hover::Fill(theme.hover_overlay);
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
    type Element = ButtonElem;

    fn apply(self, button: &mut ButtonElem, theme: &EditorTheme) {
        button.height = px(24);
        button.flex_grow = 1.0;
        button.fill = if self.active {
            theme.accent
        } else {
            theme.button_fill
        };
        button.hover = if self.active {
            Hover::None
        } else {
            Hover::Fill(theme.hover_overlay)
        };
    }
}

/// A button with no surface of its own until the cursor is on it, for
/// one that sits in a row of its own kind or on something that is
/// already a surface.
pub struct GhostButton;

impl Style for GhostButton {
    type Host = FynixHost;
    type Element = ButtonElem;

    fn apply(self, button: &mut ButtonElem, theme: &EditorTheme) {
        button.fill = Color::NONE;
        button.hover = Hover::Fill(theme.hover_overlay);
        button.padding = UiRect::axes(px(8), px(4));
        button.radius = px(4);
    }
}

impl ButtonElem {
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
            // `label().color()` hops through the `Option`
            // transparently, so its base is `Color`. A label with no
            // colour of its own has nowhere for that hop to land.
            if let Some(color) =
                self.label.as_ref().and_then(|label| label.color)
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

/// Overrides `radius` when set; leaves it be when `None`, so the two
/// can be patched independently.
fn corners(
    patch: &mut Patch<FynixHost>,
    corners: &Option<BorderRadius>,
) {
    if let Some(radius) = *corners {
        node(patch, move |n| n.border_radius = radius);
    }
}
