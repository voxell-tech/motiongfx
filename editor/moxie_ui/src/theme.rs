//! Editor theme: the raw Monokai Pro palette plus the semantic slots
//! the UI actually reads (text, accent, glass tints).
//!
//! Palette mirrors
//! `examples/bevy_examples/assets/typst/monokai_pro.typ`
//! so typst-rendered content and the editor chrome share one look.
//! Swap the [`EditorTheme`] resource to re-theme; glass materials are
//! rebuilt from it at plugin build time.

use bevy::prelude::*;

use crate::palette;

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
            red: palette::RED,
            orange: palette::ORANGE,
            yellow: palette::YELLOW,
            green: palette::GREEN,
            blue: palette::BLUE,
            purple: palette::PURPLE,
            base: palette::BASE,
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
    pub hot: Color,
}

impl Default for EditorTheme {
    fn default() -> Self {
        let palette = Palette::default();
        Self {
            text_primary: palette.base[8],
            text_muted: palette.base[6],
            accent: palette.blue,
            hover_fill: palette.base[8].with_alpha(0.06),
            hot: palette.red,
            palette,
        }
    }
}
