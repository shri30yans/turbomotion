//! The film's design tokens .

use skia_safe::Color;

pub const PAPER: Color = Color::from_rgb(0xF4, 0xEB, 0xD9);
pub const INK: Color = Color::from_rgb(0x2B, 0x1E, 0x16);
pub const INK_SOFT: Color = Color::from_rgb(0x6E, 0x5A, 0x47);
pub const TERRACOTTA: Color = Color::from_rgb(0xB4, 0x48, 0x2F);
pub const OCHRE: Color = Color::from_rgb(0xE0, 0xA9, 0x3B);
pub const TEAL: Color = Color::from_rgb(0x1F, 0x6F, 0x68);
pub const NIGHT: Color = Color::from_rgb(0x16, 0x11, 0x0D);
pub const WHITE: Color = Color::from_rgb(0xFF, 0xF8, 0xEC);

/// Black at `alpha` (0–255).
pub const fn black(alpha: u8) -> Color {
    Color::from_argb(alpha, 0, 0, 0)
}

/// `WHITE` at `alpha` (0–255).
pub const fn white(alpha: u8) -> Color {
    Color::from_argb(alpha, 0xFF, 0xF8, 0xEC)
}

pub const WATER: Color = Color::from_rgb(0x3E, 0x7F, 0xA0);
pub const LINE: Color = Color::from_rgb(0xCD, 0xB9, 0x95);

/// A `#RRGGBB` colour.
pub const fn hex(rgb: u32) -> Color {
    Color::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}
