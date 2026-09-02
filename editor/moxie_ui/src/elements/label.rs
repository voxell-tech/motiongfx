use crate::reactive::{FynixBuild, FynixHost};
use bevy::feathers::theme::ThemedText;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut;
use fynix::element::element;
use fynix::ui::Patch;

use super::patch::with;

/// A theme-inheriting text label.
#[element(build = Self::build)]
pub struct Label {
    #[elem(patch = text)]
    pub text: String,
    #[elem(patch = size)]
    #[default(12.0)]
    pub size: f32,
    /// `None` leaves the colour to the theme.
    #[elem(patch = color)]
    pub color: Option<Color>,
    #[elem(patch = bold)]
    pub bold: bool,
    #[elem(patch = wrap)]
    #[default(true)]
    pub wrap: bool,
}

fn font(size: f32, bold: bool) -> TextFont {
    TextFont {
        font_size: FontSize::Px(size),
        weight: if bold {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        },
        ..default()
    }
}

fn layout(wrap: bool) -> TextLayout {
    TextLayout {
        linebreak: if wrap {
            LineBreak::WordBoundary
        } else {
            LineBreak::NoWrap
        },
        ..default()
    }
}

impl Label {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            Text::new(self.text.clone()),
            font(self.size, self.bold),
            layout(self.wrap),
        ));

        set_color(self.color, build);
    }
}

fn text(patch: &mut Patch<FynixHost>, text: &String) {
    with::<Text>(patch, |t| t.0.clone_from(text));
}

fn size(patch: &mut Patch<FynixHost>, size: &f32) {
    with::<TextFont>(patch, |f| f.font_size = FontSize::Px(*size));
}

fn bold(patch: &mut Patch<FynixHost>, bold: &bool) {
    let weight = if *bold {
        FontWeight::BOLD
    } else {
        FontWeight::NORMAL
    };
    with::<TextFont>(patch, move |f| f.weight = weight);
}

fn wrap(patch: &mut Patch<FynixHost>, wrap: &bool) {
    patch.insert(layout(*wrap));
}

fn color(patch: &mut Patch<FynixHost>, color: &Option<Color>) {
    set_color(*color, patch);
}

/// A colour of its own opts out of the theme's.
fn set_color(color: Option<Color>, entity: &mut impl WorldEntityMut) {
    match color {
        Some(color) => entity.insert(TextColor(color)),
        None => entity.insert(ThemedText),
    };
}
