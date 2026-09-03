//! Editor theme: the raw Monokai Pro palette plus the semantic slots
//! the UI reads, grouped by what they govern.
//!
//! Palette mirrors
//! `examples/bevy_examples/assets/typst/monokai_pro.typ`
//! so typst-rendered content and the editor chrome share one look.

use std::time::Duration;

use bevy::prelude::*;
use motiongfx_interp::ease::{self, EaseFn};

/// The editor's ground colour, also [`Colors::bg`].
pub const BG: Color = Color::srgb_u8(0x19, 0x18, 0x1A);

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
                BG,
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

/// The editor's look, grouped by what it governs.
#[derive(Clone, Debug)]
pub struct EditorTheme {
    pub palette: Palette,
    pub color: Colors,
    pub space: Spacing,
    pub text: TextScale,
    pub motion: Motion,
}

/// Semantic colour slots. A fill is translucent and layers over
/// whatever is behind it, a ground is opaque.
#[derive(Clone, Copy, Debug)]
pub struct Colors {
    /// Primary (active) text.
    pub text: Color,
    /// Secondary / inactive text and icons.
    pub text_dim: Color,
    /// A third, fainter tier, for a de-emphasised label beside a
    /// brighter one.
    pub text_faint: Color,
    /// Interactive accent.
    pub accent: Color,
    /// Destructive accents, and a draft or error state.
    pub critical: Color,
    /// The editor's own ground.
    pub bg: Color,
    /// Panels and popups.
    pub panel: Color,
    /// A raised strip within a panel.
    pub surface: Color,
    /// A filled control's resting surface.
    pub fill: Color,
    /// A barely-there fill, for a tint rather than a surface.
    pub fill_faint: Color,
    /// Dividers and borders.
    pub hairline: Color,
    /// The overlay a plain surface fades to under the cursor.
    pub hover: Color,
    /// A selected row's surface tint.
    pub selection: Color,
    /// A timeline clip's fill.
    pub clip: Color,
    /// A clip's own hover and press brighten, in its blue family
    /// rather than [`Self::hover`]'s neutral gray.
    pub clip_hover: Color,
    pub clip_press: Color,
}

/// The spacing and sizing scale.
#[derive(Clone, Copy, Debug)]
pub struct Spacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    /// The default corner.
    pub radius: f32,
    /// The standard height of a row or an interactive control.
    pub row: f32,
    /// A toolbar button's square.
    pub touch: f32,
    /// The default icon.
    pub icon: f32,
    /// A divider or rail's thickness.
    pub hairline: f32,
    /// A fold's chevron, sized to sit beside a row.
    pub fold_toggle: f32,
    /// How far a fold's rail sets its body in from the header.
    pub fold_indent: f32,
}

/// Font sizes, three steps.
#[derive(Clone, Copy, Debug)]
pub struct TextScale {
    pub small: f32,
    pub body: f32,
    pub label: f32,
}

/// How the UI moves.
#[derive(Clone, Copy, Debug)]
pub struct Motion {
    /// How long a hover or press fade takes.
    pub interact: Duration,
    /// The curve an interaction fade follows.
    pub ease: EaseFn,
}

impl Default for EditorTheme {
    fn default() -> Self {
        let palette = Palette::default();
        let base = palette.base;
        Self {
            color: Colors {
                text: base[8],
                text_dim: base[6],
                text_faint: base[8].with_alpha(0.6),
                accent: palette.blue,
                critical: palette.red,
                bg: base[0],
                panel: base[1],
                surface: base[2],
                fill: base[8].with_alpha(0.06),
                fill_faint: base[8].with_alpha(0.03),
                hairline: base[8].with_alpha(0.08),
                hover: base[8].with_alpha(0.14),
                selection: palette.blue.with_alpha(0.18),
                clip: palette.blue.with_alpha(0.5),
                clip_hover: Color::srgb(0.35, 0.70, 1.0),
                clip_press: Color::srgb(0.55, 0.82, 1.0),
            },
            space: Spacing {
                xs: 2.0,
                sm: 4.0,
                md: 6.0,
                lg: 8.0,
                xl: 12.0,
                radius: 4.0,
                row: 24.0,
                touch: 26.0,
                icon: 11.0,
                hairline: 1.0,
                fold_toggle: 14.0,
                fold_indent: 9.0,
            },
            text: TextScale {
                small: 10.0,
                body: 12.0,
                label: 14.0,
            },
            motion: Motion {
                interact: Duration::from_millis(120),
                ease: ease::cubic::ease_out,
            },
            palette,
        }
    }
}
