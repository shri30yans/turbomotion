//! Colour helpers shared by elements.

use skia_safe::{Color, Color4f, Paint};

use super::model::Rgb;

/// Blend `a` toward `b` by `k`, truncating each channel to an integer as the reference `mix` does.
pub fn mix(a: Rgb, b: Rgb, k: f64) -> Rgb {
    std::array::from_fn(|i| (f64::from(a[i]) + (f64::from(b[i]) - f64::from(a[i])) * k) as u8)
}

pub fn skia(c: Rgb) -> Color {
    Color::from_rgb(c[0], c[1], c[2])
}

pub fn skia_a(c: Rgb, alpha: f64) -> Color4f {
    Color4f::new(f32::from(c[0]) / 255.0, f32::from(c[1]) / 255.0, f32::from(c[2]) / 255.0, alpha as f32)
}

/// An anti-aliased solid fill.
pub fn fill(c: Rgb) -> Paint {
    let mut p = Paint::default();
    p.set_anti_alias(true).set_color(skia(c));
    p
}

/// An anti-aliased solid fill with fractional opacity.
pub fn fill_a(c: Rgb, alpha: f64) -> Paint {
    let mut p = Paint::default();
    p.set_anti_alias(true).set_color4f(skia_a(c, alpha), None);
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_truncates_like_the_reference() {
        assert_eq!(mix([246, 244, 238], [0, 0, 0], 0.62), [93, 92, 90]);
        assert_eq!(mix([10, 20, 30], [255, 255, 255], 0.35), [95, 102, 108]);
    }
}
