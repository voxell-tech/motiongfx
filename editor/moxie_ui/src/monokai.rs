//! The raw Monokai Pro colours as `const`s, for what
//! [`Palette`](crate::theme::Palette) cannot reach: a
//! [`Style`](fynix::style::Style) writes fields before a node
//! exists, with no [`World`](bevy::prelude::World) to read
//! [`EditorTheme`](crate::theme::EditorTheme) from. `Palette` builds
//! its fields from these, so the two never drift apart.

use bevy::prelude::Color;

pub const RED: Color = Color::srgb_u8(0xFF, 0x61, 0x88);
pub const ORANGE: Color = Color::srgb_u8(0xFC, 0x98, 0x67);
pub const YELLOW: Color = Color::srgb_u8(0xFF, 0xD8, 0x66);
pub const GREEN: Color = Color::srgb_u8(0xA9, 0xDC, 0x76);
pub const BLUE: Color = Color::srgb_u8(0x78, 0xDC, 0xE8);
pub const PURPLE: Color = Color::srgb_u8(0xAB, 0x9D, 0xF2);

/// Darkest → lightest neutrals.
pub const BASE: [Color; 9] = [
    Color::srgb_u8(0x19, 0x18, 0x1A),
    Color::srgb_u8(0x22, 0x1F, 0x22),
    Color::srgb_u8(0x2D, 0x2A, 0x2E),
    Color::srgb_u8(0x40, 0x3E, 0x41),
    Color::srgb_u8(0x5B, 0x59, 0x5C),
    Color::srgb_u8(0x72, 0x70, 0x72),
    Color::srgb_u8(0x93, 0x92, 0x93),
    Color::srgb_u8(0xC1, 0xC0, 0xC0),
    Color::srgb_u8(0xFC, 0xFC, 0xFA),
];
