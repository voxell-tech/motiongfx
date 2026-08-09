use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::Button as ButtonBehavior;
use bevy::window::SystemCursorIcon;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

use super::Icon;

/// How a button is dressed. Only the look differs: either one is a
/// button, and behaves as one.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonLook {
    /// A filled pill.
    #[default]
    Normal,
    /// Nothing of its own, so the hit area is all there is around
    /// whatever the button holds.
    Ghost,
}

/// A hit area holding an icon, or whatever is built under it.
#[derive(Element, OverrideDefault, Lenz)]
pub struct Button {
    /// A node of its own, so its image and colour can be bound
    /// without touching the button.
    #[elem]
    pub icon: Option<Icon>,
    pub look: ButtonLook,
    /// What [`ButtonLook::Normal`] fills with.
    #[default(Color::srgba(1.0, 1.0, 1.0, 0.06))]
    pub fill: Color,
    /// The hit area, which an icon button wants square and a button
    /// with a word in it does not.
    #[default(Val::Px(18.0))]
    pub width: Val,
    #[default(Val::Px(18.0))]
    pub height: Val,
    /// Centred, for a button that is only as big as what it holds.
    #[default(::Center)]
    pub justify: JustifyContent,
    pub padding: UiRect,
    #[default(Val::ZERO)]
    pub radius: Val,
}

impl Button {
    fn node(&self) -> Node {
        Node {
            width: self.width,
            height: self.height,
            justify_content: self.justify,
            align_items: AlignItems::Center,
            padding: self.padding,
            border_radius: BorderRadius::all(self.radius),
            ..default()
        }
    }

    /// What the look brings, which is a surface or nothing.
    fn background(&self) -> BackgroundColor {
        match self.look {
            ButtonLook::Normal => BackgroundColor(self.fill),
            ButtonLook::Ghost => BackgroundColor(Color::NONE),
        }
    }
}

impl ElementVisual<BevyHost> for Button {
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
        field: ButtonField,
    ) {
        let mut entity = world.entity_mut(node);

        match field {
            ButtonField::Look | ButtonField::Fill => {
                entity.insert(self.background());
            }
            // Every other field is one of `Node`'s, and writing it
            // whole is one insert either way.
            ButtonField::Width
            | ButtonField::Height
            | ButtonField::Justify
            | ButtonField::Padding
            | ButtonField::Radius => {
                entity.insert(self.node());
            }
        }
    }
}
