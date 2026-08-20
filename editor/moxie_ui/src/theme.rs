//! Editor theme: the raw Monokai Pro palette plus the semantic slots
//! the UI actually reads (text, accent, interaction fades).
//!
//! Palette mirrors
//! `examples/bevy_examples/assets/typst/monokai_pro.typ`
//! so typst-rendered content and the editor chrome share one look.

use bevy::prelude::*;

use crate::monokai;

/// Raw Monokai Pro palette, as the fields the rest of the editor reads
/// by name. [`palette`] carries the same colours as `const`s, for
/// wherever there is no [`EditorTheme`] resource to read this from.
#[derive(Clone, Debug)]
pub struct Palette {
    pub red: Color,
    pub orange: Color,
    pub yellow: Color,
    pub green: Color,
    pub blue: Color,
    pub purple: Color,
    /// Darkest → lightest neutrals.
    pub base: [Color; 9],
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            red: monokai::RED,
            orange: monokai::ORANGE,
            yellow: monokai::YELLOW,
            green: monokai::GREEN,
            blue: monokai::BLUE,
            purple: monokai::PURPLE,
            base: monokai::BASE,
        }
    }
}

/// Semantic colors read by the editor UI.
#[derive(Resource, Clone, Debug)]
pub struct EditorTheme {
    pub palette: Palette,
    /// Primary (active) text.
    pub text_primary: Color,
    /// Secondary / inactive text and icons.
    pub text_muted: Color,
    /// Interactive accent (active tabs, drop targets).
    pub accent: Color,
    /// Subtle hover fill for list rows and the like.
    pub hover_fill: Color,
    /// Playhead / destructive accents.
    pub critical: Color,
    /// The faint surface a filled button rests at.
    pub button_fill: Color,
    /// What a plain surface fades to under the cursor.
    pub hover_overlay: Color,
    /// What it fades to further while held.
    pub press_overlay: Color,
    /// How long a hover/press fade takes.
    pub interact_ms: u32,
    /// What a timeline clip brightens to under the cursor. Its own
    /// family of color, not [`Self::hover_overlay`]'s neutral gray.
    pub clip_hover: Color,
    /// Same, further while held.
    pub clip_press: Color,
}

impl Default for EditorTheme {
    fn default() -> Self {
        let palette = Palette::default();
        Self {
            text_primary: palette.base[8],
            text_muted: palette.base[6],
            accent: palette.blue,
            hover_fill: palette.base[8].with_alpha(0.06),
            critical: palette.red,
            button_fill: palette.base[8].with_alpha(0.06),
            hover_overlay: palette.base[8].with_alpha(0.14),
            press_overlay: palette.base[8].with_alpha(0.22),
            interact_ms: 120,
            clip_hover: Color::srgb(0.35, 0.70, 1.0),
            clip_press: Color::srgb(0.55, 0.82, 1.0),
            palette,
        }
    }
}
