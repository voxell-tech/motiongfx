use crate::reactive::BevyHost;
use bevy::feathers::theme::ThemedText;
use bevy::prelude::*;
use bevy_fynix::EntityExt as _;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::{Build, Patch};

/// A theme-inheriting text label.
#[derive(Element)]
pub struct Label {
    pub text: String,
    #[default(12.0)]
    pub size: f32,
    /// `None` leaves the colour to the theme.
    pub color: Option<Color>,
    pub bold: bool,
    /// Off for a label in a row that must not reflow.
    #[default(true)]
    pub wrap: bool,
}

impl Label {
    fn font(&self) -> TextFont {
        TextFont {
            font_size: FontSize::Px(self.size),
            weight: if self.bold {
                FontWeight::BOLD
            } else {
                FontWeight::NORMAL
            },
            ..default()
        }
    }

    fn layout(&self) -> TextLayout {
        TextLayout {
            linebreak: if self.wrap {
                LineBreak::WordBoundary
            } else {
                LineBreak::NoWrap
            },
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for Label {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((
            Text::new(self.text.clone()),
            self.font(),
            self.layout(),
        ));

        // A colour of its own opts out of the theme's.
        match self.color {
            Some(color) => build.insert(TextColor(color)),
            None => build.insert(ThemedText),
        };
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: LabelField,
    ) {
        let node = patch.id();
        let world = &mut *patch.world;

        let mut node = world.entity_mut(node);

        match field {
            LabelField::Text => {
                if let Some(mut text) = node.get_mut::<Text>() {
                    text.0.clone_from(&self.text);
                }
            }
            LabelField::Size | LabelField::Bold => {
                node.insert(self.font());
            }
            LabelField::Wrap => {
                node.insert(self.layout());
            }
            LabelField::Color => {
                match self.color {
                    Some(color) => node.insert(TextColor(color)),
                    None => node.insert(ThemedText),
                };
            }
        }
    }
}
