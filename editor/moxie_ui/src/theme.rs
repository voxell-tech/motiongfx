//! Editor theme: the raw Monokai Pro palette plus the semantic slots
//! the UI actually reads (text, accent, glass tints).
//!
//! Palette mirrors
//! `examples/bevy_examples/assets/typst/monokai_pro.typ`
//! so typst-rendered content and the editor chrome share one look.
//! Swap the [`EditorTheme`] resource to re-theme; glass materials are
//! rebuilt from it at plugin build time.

use bevy::prelude::*;

/// Raw Monokai Pro palette.
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
            red: Color::srgb_u8(0xFF, 0x61, 0x88),
            orange: Color::srgb_u8(0xFC, 0x98, 0x67),
            yellow: Color::srgb_u8(0xFF, 0xD8, 0x66),
            green: Color::srgb_u8(0xA9, 0xDC, 0x76),
            blue: Color::srgb_u8(0x78, 0xDC, 0xE8),
            purple: Color::srgb_u8(0xAB, 0x9D, 0xF2),
            base: [
                Color::srgb_u8(0x19, 0x18, 0x1A),
                Color::srgb_u8(0x22, 0x1F, 0x22),
                Color::srgb_u8(0x2D, 0x2A, 0x2E),
                Color::srgb_u8(0x40, 0x3E, 0x41),
                Color::srgb_u8(0x5B, 0x59, 0x5C),
                Color::srgb_u8(0x72, 0x70, 0x72),
                Color::srgb_u8(0x93, 0x92, 0x93),
                Color::srgb_u8(0xC1, 0xC0, 0xC0),
                Color::srgb_u8(0xFC, 0xFC, 0xFA),
            ],
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
