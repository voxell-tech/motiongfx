use crate::reactive::BevyHost;
use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::Button as ButtonBehavior;
use bevy::window::SystemCursorIcon;
use bevy_fynix::EntityExt as _;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::style::Style;
use fynix_mock::ui::{Build, Patch};

use super::{Icon, IconCursor, Label, LabelCursor};
use crate::motion::{self, LitFrom as _};
use crate::theme::EditorTheme;

/// The faint surface a filled button rests at.
const FILL: Color = Color::srgba(1.0, 1.0, 1.0, 0.06);

/// A tinted button's icon and label colour.
const TINT: Color = crate::monokai::BLUE;

/// What lights up under the cursor, and to what colour. A style has
/// no node to wire this on, so it leaves the choice here for
/// [`build_fields`](ElementVisual::build_fields) to read once the
/// node exists.
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
/// under it. Undressed: [`Button`] and [`GhostButton`] are the two
/// looks the editor gives it.
#[derive(Element)]
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
    pub column_gap: Val,
    /// What the background shows. Nothing by default, which is a
    /// [`GhostButton`]; [`Button`] rests at `FILL`, and interaction
    /// lights either of them up.
    #[default(::NONE)]
    pub fill: Color,
    pub width: Val,
    pub height: Val,
    #[default(px(18))]
    pub min_width: Val,
    #[default(px(18))]
    pub min_height: Val,
    /// Centred, for a button that is only as big as what it holds.
    #[default(::Center)]
    pub justify: JustifyContent,
    pub padding: UiRect,
    #[default(::ZERO)]
    pub radius: Val,
    /// Set by whichever [`Style`] built this - see [`Hover`]. Never
    /// patched: read once, in `build_fields`.
    #[elem(ignore)]
    hover: Hover,
}

impl ButtonElem {
    fn node(&self) -> Node {
        Node {
            min_width: self.min_width,
            min_height: self.min_height,
            width: self.width,
            height: self.height,
            justify_content: self.justify,
            align_items: AlignItems::Center,
            column_gap: self.column_gap,
            padding: self.padding,
            border_radius: BorderRadius::all(self.radius),
            ..default()
        }
    }

    /// `fill` alone, so a lane aiming it under the cursor takes
    /// effect whichever look the button wears.
    fn background(&self) -> BackgroundColor {
        BackgroundColor(self.fill)
    }
}

/// The editor's own button: a filled, rounded pill sized for a
/// toolbar, which lights up under the cursor. A call site that holds a
/// word rather than an icon says so by resizing it.
pub struct Button;

impl Style for Button {
    type Host = BevyHost;
    type Element = ButtonElem;

    fn apply(self, button: &mut ButtonElem, _theme: &EditorTheme) {
        button.fill = FILL;
        button.width = px(26);
        button.height = px(26);
        button.radius = px(6);
        button.hover = Hover::Fill(motion::HOVER);
    }
}

/// A button whose icon and label carry `tint` under the cursor - the
/// accent, by default, for the one action in a group the eye should
/// land on first.
pub struct TintButton {
    pub tint: Color,
}

impl Default for TintButton {
    fn default() -> Self {
        Self { tint: TINT }
    }
}

impl Style for TintButton {
    type Host = BevyHost;
    type Element = ButtonElem;

    fn apply(self, button: &mut ButtonElem, _theme: &EditorTheme) {
        button.hover = Hover::IconLabel(self.tint);
    }
}

/// A menu bar's own button: square, full height, and lighting up only
/// under the cursor, so a row of them reads as one strip rather than a
/// row of separate controls.
pub struct MenuButton;

impl Style for MenuButton {
    type Host = BevyHost;
    type Element = ButtonElem;

    fn apply(self, button: &mut ButtonElem, _theme: &EditorTheme) {
        button.fill = Color::NONE;
        button.radius = Val::ZERO;
        button.height = percent(100);
        button.padding = UiRect::axes(px(10), Val::ZERO);
        button.hover = Hover::Fill(motion::HOVER);
    }
}

/// A button with no surface of its own until the cursor is on it, for
/// one that sits in a row of its own kind or on something that is
/// already a surface.
pub struct GhostButton;

impl Style for GhostButton {
    type Host = BevyHost;
    type Element = ButtonElem;

    fn apply(self, button: &mut ButtonElem, _theme: &EditorTheme) {
        button.fill = Color::NONE;
        button.hover = Hover::Fill(motion::HOVER);
    }
}

impl ElementVisual<BevyHost> for ButtonElem {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((
            self.node(),
            self.background(),
            ButtonBehavior,
            EntityCursor::System(SystemCursorIcon::Pointer),
        ));

        // What the style asked for, wired against a base this method
        // already has as `&self` - the node has no entry in the
        // kernel's own table yet for `.lit()` to read one from. Both
        // stops light to the same colour: `Hover` carries one, not a
        // separate hover and press shade.
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
            // Each read straight from `self`, not through the
            // cursor: absent is skipped rather than defaulted, the
            // way the field simply not lighting up would read if
            // `icon`/`label` were never there to begin with.
            if let Some(icon) = &self.icon {
                build.lit_from(
                    |button| button.icon().color(),
                    icon.color,
                    tint,
                    tint,
                );
            }
            // `label().color()` hops through an `Option`
            // transparently, so its base is `Color`, not
            // `Option<Color>` - and a label with no colour of its own
            // has nowhere for that hop to land, same as before.
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

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: ButtonElemField,
    ) {
        let node = patch.id();
        let world = &mut *patch.world;

        let mut entity = world.entity_mut(node);

        match field {
            ButtonElemField::Fill => {
                entity.insert(self.background());
            }
            // Every other field is one of `Node`'s, and writing it
            // whole is one insert either way.
            ButtonElemField::MinWidth
            | ButtonElemField::MinHeight
            | ButtonElemField::Width
            | ButtonElemField::Height
            | ButtonElemField::Justify
            | ButtonElemField::ColumnGap
            | ButtonElemField::Padding
            | ButtonElemField::Radius => {
                entity.insert(self.node());
            }
        }
    }
}
