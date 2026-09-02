use crate::reactive::FynixBuild;
use bevy::feathers::theme::ThemedText;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut;
use fynix::element::element;

use super::patch::*;

/// A theme-inheriting text label.
#[element(build = Self::build)]
pub struct Label {
    #[elem(patch = PatchText)]
    pub text: String,
    #[elem(patch = PatchTextSize)]
    #[default(12.0)]
    pub size: f32,
    /// `None` leaves the colour to the theme.
    #[elem(patch = PatchTextColor)]
    pub color: Option<Color>,
    #[elem(patch = PatchBold)]
    pub bold: bool,
    #[elem(patch = PatchWrap)]
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

field_patch!(PatchText, String, |patch, v| {
    with::<Text>(patch, |t| t.0.clone_from(v));
});

field_patch!(PatchTextSize, f32, |patch, v| {
    with::<TextFont>(patch, |f| f.font_size = FontSize::Px(*v));
});

field_patch!(PatchBold, bool, |patch, v| {
    let weight = if *v {
        FontWeight::BOLD
    } else {
        FontWeight::NORMAL
    };
    with::<TextFont>(patch, move |f| f.weight = weight);
});

field_patch!(PatchWrap, bool, |patch, v| {
    patch.insert(layout(*v));
});

field_patch!(PatchTextColor, Option<Color>, |patch, v| {
    set_color(*v, patch);
});

/// A colour of its own opts out of the theme's.
fn set_color(color: Option<Color>, entity: &mut impl WorldEntityMut) {
    match color {
        Some(color) => entity.insert(TextColor(color)),
        None => entity.insert(ThemedText),
    };
}
