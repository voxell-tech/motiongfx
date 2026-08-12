use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::Button as ButtonBehavior;
use bevy::window::SystemCursorIcon;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;
use fynix_mock::style::Style;
use fynix_mock::ui::ElementMut;

use super::{Icon, IconCursor, Label, LabelCursor};
use crate::motion::{self, MotionExt};

/// The faint surface a filled button rests at.
const FILL: Color = Color::srgba(1.0, 1.0, 1.0, 0.06);

/// A tinted button's icon and label colour.
const TINT: Color = crate::palette::BLUE;

/// A hit area holding an icon, a label, both, or whatever is built
/// under it. Undressed: [`Button`] and [`GhostButton`] are the two
/// looks the editor gives it.
#[derive(Element, OverrideDefault, Lenz)]
pub struct ButtonElem {
    /// A node of its own, so its image and colour can be bound
    /// without touching the button.
    #[elem]
    pub icon: Option<Icon>,
    /// A node of its own, so its text can be bound without touching
    /// the button.
    #[elem]
    pub label: Option<Label>,
    /// Between the icon and the label, when both are there.
    #[default(::ZERO)]
    pub column_gap: Val,
    /// What the background shows. Nothing by default, which is a
    /// [`GhostButton`]; [`Button`] rests at [`FILL`], and interaction
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
    #[default(Val::ZERO)]
    pub radius: Val,
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

    fn apply(self, button: &mut ButtonElem) {
        button.fill = FILL;
        button.width = px(26);
        button.height = px(26);
        button.radius = px(6);
    }

    fn attach(elem: ElementMut<BevyHost, ButtonElem>) {
        lit(elem);
    }
}

/// A button whose icon and label carry the accent under the cursor,
/// for the one action in a group the eye should land on first.
pub struct TintButton;

impl Style for TintButton {
    type Host = BevyHost;
    type Element = ButtonElem;

    fn attach(elem: ElementMut<BevyHost, ButtonElem>) {
        elem.lit(|button| button.icon().color(), TINT, TINT).lit(
            |button| button.label().color(),
            TINT,
            TINT,
        );
    }
}

/// A button with no surface of its own until the cursor is on it, for
/// one that sits in a row of its own kind or on something that is
/// already a surface.
pub struct GhostButton;

impl Style for GhostButton {
    type Host = BevyHost;
    type Element = ButtonElem;

    fn apply(self, button: &mut ButtonElem) {
        button.fill = Color::NONE;
    }

    fn attach(elem: ElementMut<BevyHost, ButtonElem>) {
        lit(elem);
    }
}

/// Every look lights up the same way.
fn lit<'u, 'a>(
    elem: ElementMut<'u, 'a, BevyHost, ButtonElem>,
) -> ElementMut<'u, 'a, BevyHost, ButtonElem> {
    elem.lit(|button| button.fill(), motion::HOVER, motion::PRESS)
}

impl ElementVisual<BevyHost> for ButtonElem {
    fn build_fields(&self, world: &mut World, node: Entity) {
        let mut entity = world.entity_mut(node);

        entity.insert((
            self.node(),
            self.background(),
            ButtonBehavior,
            EntityCursor::System(SystemCursorIcon::Pointer),
        ));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: ButtonElemField,
    ) {
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
