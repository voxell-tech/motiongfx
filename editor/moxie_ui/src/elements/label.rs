use crate::reactive::FynixBuild;
use bevy::feathers::theme::ThemedText;
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut;
use bevy_fynix::tag::Hovered;
use fynix::element::element;

use super::patch::*;

/// A theme-inheriting text label.
#[element(build = Self::build)]
pub struct Label {
    #[elem(patch = PatchText)]
    pub text: String,
    #[elem(default = theme.text.body, patch = PatchTextSize)]
    pub size: f32,
    /// [`Color::NONE`] leaves the colour to the theme.
    #[elem(default = ::NONE, patch = PatchTextColor, anim(
        duration = theme.motion.interact,
        ease = theme.motion.ease,
        on(Hovered, read = Self::lit),
    ))]
    pub color: Color,
    /// What `color` travels to while the pointer is over whatever
    /// owns this label; `None` leaves it at rest. Element state: the
    /// lines read it, nothing draws it.
    pub hover_color: Option<Color>,
    #[elem(patch = PatchBold)]
    pub bold: bool,
    #[elem(default = true, patch = PatchWrap)]
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
    /// Where `color` heads under the cursor: the tint if one was
    /// set, otherwise its own resting colour, so nothing moves.
    fn lit(&self) -> &Color {
        match &self.hover_color {
            Some(color) => color,
            None => &self.color,
        }
    }

    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        // Invisible to picking, for the same reason as [`Icon`]: the
        // parent widget's hit area owns the pointer.
        build.insert((
            Text::new(self.text.clone()),
            font(self.size, self.bold),
            layout(self.wrap),
            Pickable {
                should_block_lower: false,
                is_hoverable: false,
            },
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

field_patch!(PatchTextColor, Color, |patch, v| {
    set_color(*v, patch);
});

/// Insert an explicit [`TextColor`], or [`ThemedText`] for
/// [`Color::NONE`].
fn set_color(color: Color, entity: &mut impl WorldEntityMut) {
    if color == Color::NONE {
        entity.insert(ThemedText);
    } else {
        entity.insert(TextColor(color));
    }
}
