//! Effects: a library of layer effects that any element or whole scene can carry as an `fx` list.
//!
//! ```json
//! {"k": "title", "text": "HELLO", "fx": [{"effect": "glow", "radius": 30}, {"effect": "blur", "radius": [[2, 24], [3, 0]]}]}
//! ```
//!
//! Each effect takes the layer built so far and returns a new one, in list order. Numbers may be animated with keys
//! `[[seconds, value], ...]`. `turbomotion studio effects` prints the catalogue; the registry below is its only source,
//! so the documentation cannot drift from what the renderer accepts.

mod blend;
mod blur;
mod color;
mod distort;
mod edge;
mod param;
mod pass;
mod place;
mod programs;
mod texture;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use skia_safe::{Canvas, Paint};

pub use blend::Blend;
pub use pass::Ctx;
pub use programs::Programs;

use super::{layer::Layer, stage::Stage};

/// One catalogue entry: what the effect is called, what it does and a complete example with every parameter.
#[derive(Debug, Serialize)]
pub struct Entry {
    pub effect: &'static str,
    pub summary: &'static str,
    pub example: serde_json::Value,
}

macro_rules! registry {
    ($( $variant:ident($ty:ty) = $name:literal, $func:path, $summary:literal, $example:literal; )*) => {
        /// An effect and its parameters, selected by `"effect": "<name>"`.
        #[derive(Debug, Deserialize, Clone)]
        #[serde(tag = "effect")]
        pub enum Effect {
            $( #[serde(rename = $name)] $variant($ty), )*
        }

        impl Effect {
            pub fn name(&self) -> &'static str {
                match self { $( Self::$variant(_) => $name, )* }
            }

            fn run(&self, ctx: &Ctx, layer: &Layer) -> Result<Layer> {
                match self { $( Self::$variant(e) => $func(ctx, e, layer), )* }
            }
        }

        /// Every effect, in catalogue order.
        pub fn catalogue() -> Vec<Entry> {
            vec![ $( Entry { effect: $name, summary: $summary, example: serde_json::from_str($example).expect("catalogue example is JSON") }, )* ]
        }
    };
}

registry! {
    Blur(blur::Blur) = "blur", blur::blur, "Gaussian blur. `radius` is the sigma in pixels.", r#"{"effect": "blur", "radius": 8}"#;
    DirectionalBlur(blur::DirectionalBlur) = "directional_blur", blur::directional_blur, "Motion blur: smears the layer `distance` pixels along `angle` degrees.", r#"{"effect": "directional_blur", "distance": 40, "angle": 0}"#;
    RadialBlur(blur::RadialBlur) = "radial_blur", blur::radial_blur, "Zoom blur from a point (`x`, `y` as fractions of the frame); `amount` 0.1 smears 10%.", r#"{"effect": "radial_blur", "amount": 0.1, "x": 0.5, "y": 0.5}"#;
    Glow(blur::Glow) = "glow", blur::glow, "Adds a blurred copy back on top. `threshold` limits it to the bright parts, `color` tints the halo.", r#"{"effect": "glow", "radius": 24, "strength": 1, "threshold": 0, "color": [255, 200, 120]}"#;
    Shadow(edge::Shadow) = "shadow", edge::shadow, "Soft drop shadow behind the layer's shape.", r#"{"effect": "shadow", "dx": 0, "dy": 12, "blur": 16, "color": [0, 0, 0], "opacity": 0.5}"#;
    Outline(edge::Outline) = "outline", edge::outline, "Solid outline `width` pixels around the layer's shape.", r#"{"effect": "outline", "width": 4, "color": [255, 255, 255], "opacity": 1}"#;
    Brightness(color::Brightness) = "brightness", color::brightness, "Adds `amount` (0..1 scale) to every channel.", r#"{"effect": "brightness", "amount": 0.2}"#;
    Contrast(color::Contrast) = "contrast", color::contrast, "1 = unchanged, 0 = flat grey, 2 = punchy.", r#"{"effect": "contrast", "amount": 1.3}"#;
    Saturation(color::Saturation) = "saturation", color::saturation, "1 = unchanged, 0 = black and white, 2 = vivid.", r#"{"effect": "saturation", "amount": 1.4}"#;
    Hue(color::Hue) = "hue", color::hue, "Rotates colours around the wheel by `degrees`.", r#"{"effect": "hue", "degrees": 90}"#;
    Exposure(color::Exposure) = "exposure", color::exposure, "Photographic stops: +1 doubles the light.", r#"{"effect": "exposure", "amount": 0.5}"#;
    Gamma(color::Gamma) = "gamma", color::gamma, "Mid-tone curve: above 1 lightens, below 1 darkens.", r#"{"effect": "gamma", "amount": 1.2}"#;
    Invert(color::Invert) = "invert", color::invert, "Negative; `amount` 1 is a full invert.", r#"{"effect": "invert", "amount": 1}"#;
    Tint(color::Tint) = "tint", color::tint, "Paints the layer toward `color` by `amount` (0..1), keeping its shape.", r#"{"effect": "tint", "color": [255, 120, 60], "amount": 0.6}"#;
    Grade(crate::studio::model::Grade) = "grade", color::grade, "The scene colour grade on one layer: lift, gamma, gain, sat, contrast, tint (rgb) and tint_k.", r#"{"effect": "grade", "lift": 0, "gamma": 1, "gain": 1, "sat": 0.8, "contrast": 1.1, "tint": [20, 40, 80], "tint_k": 0.1}"#;
    Wave(distort::Wave) = "wave", distort::wave, "Sine wave displacement: `amplitude` px, `wavelength` px, `speed` crests per second, `angle` of travel.", r#"{"effect": "wave", "amplitude": 12, "wavelength": 160, "speed": 0.5, "angle": 0}"#;
    Twirl(distort::Twirl) = "twirl", distort::twirl, "Swirls the picture around (`x`, `y`) by `angle` degrees within `radius` pixels.", r#"{"effect": "twirl", "angle": 90, "radius": 300, "x": 0.5, "y": 0.5}"#;
    Bulge(distort::Bulge) = "bulge", distort::bulge, "Magnifies (positive) or pinches (negative) the picture around (`x`, `y`) within `radius` pixels.", r#"{"effect": "bulge", "strength": 0.5, "radius": 300, "x": 0.5, "y": 0.5}"#;
    Pixelate(distort::Pixelate) = "pixelate", distort::pixelate, "Mosaic of `size` pixel blocks.", r#"{"effect": "pixelate", "size": 16}"#;
    ChromaticAberration(distort::ChromaticAberration) = "chromatic_aberration", distort::chromatic_aberration, "Red and blue part ways toward the corners by `amount` pixels, like a cheap lens.", r#"{"effect": "chromatic_aberration", "amount": 6}"#;
    RgbSplit(distort::RgbSplit) = "rgb_split", distort::rgb_split, "Red and blue shift `distance` pixels apart along `angle` degrees; green stays.", r#"{"effect": "rgb_split", "distance": 8, "angle": 0}"#;
    Grain(texture::Grain) = "grain", texture::grain, "Film grain, redrawn 24 times a second.", r#"{"effect": "grain", "amount": 0.12, "seed": 0}"#;
    Scanlines(texture::Scanlines) = "scanlines", texture::scanlines, "CRT scanlines every `spacing` pixels, dimming by `strength`.", r#"{"effect": "scanlines", "spacing": 4, "strength": 0.35, "thickness": 0.5}"#;
    Vignette(texture::Vignette) = "vignette", texture::vignette, "Darkens toward the corners.", r#"{"effect": "vignette", "strength": 0.5, "softness": 0.45}"#;
    Opacity(place::Opacity) = "opacity", place::opacity, "Fades the whole layer.", r#"{"effect": "opacity", "value": 0.6}"#;
    Transform(place::Transform) = "transform", place::transform, "Moves (`x`, `y` px), scales and rotates (clockwise degrees) the layer about an anchor (fractions of the frame).", r#"{"effect": "transform", "x": 40, "y": -20, "scale": 1.15, "rotation": -6, "anchor_x": 0.5, "anchor_y": 0.5}"#;
}

/// Run `chain` over `layer` at time `t`, in order.
pub fn apply(st: &Stage, like: &Canvas, layer: Layer, chain: &[Effect], t: f64) -> Result<Layer> {
    let ctx = Ctx { st, like, t };
    chain.iter().try_fold(layer, |layer, effect| effect.run(&ctx, &layer).with_context(|| format!("effect {:?}", effect.name())))
}

/// Draw `layer` onto `cv` with the element's blend mode.
pub fn composite(cv: &Canvas, layer: &Layer, blend: Option<Blend>) {
    let mut paint = Paint::default();
    paint.set_blend_mode(blend.unwrap_or(Blend::Normal).skia());
    cv.draw_image(layer.image(), (0, 0), Some(&paint));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalogue_example_parses_as_its_own_effect() {
        let entries = catalogue();
        assert!(entries.len() >= 25);
        for entry in entries {
            let effect: Effect = serde_json::from_value(entry.example.clone()).unwrap_or_else(|e| panic!("{}: {e}", entry.effect));
            assert_eq!(effect.name(), entry.effect);
        }
    }

    #[test]
    fn elements_carry_effects_and_a_blend_mode() {
        let el: crate::studio::model::Element = serde_json::from_str(r#"{"k": "text", "text": "hi", "fx": [{"effect": "blur", "radius": 4}, {"effect": "glow"}], "blend": "screen"}"#).unwrap();
        assert_eq!(el.common.fx.iter().map(Effect::name).collect::<Vec<_>>(), ["blur", "glow"]);
        assert_eq!(el.common.blend, Some(Blend::Screen));
        assert!(serde_json::from_str::<crate::studio::model::Element>(r#"{"k": "text", "text": "hi", "blend": "sparkly"}"#).is_err());
    }

    #[test]
    fn names_are_unique() {
        let mut names: Vec<_> = catalogue().iter().map(|e| e.effect).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
    }

    #[test]
    fn unknown_effects_and_parameters_are_errors() {
        assert!(serde_json::from_str::<Effect>(r#"{"effect": "sparkle"}"#).is_err());
        assert!(serde_json::from_str::<Effect>(r#"{"effect": "blur", "radis": 3}"#).is_err());
        assert!(serde_json::from_str::<Effect>(r#"{"effect": "blur", "radius": [[1, 2], [0, 3]]}"#).is_err());
    }
}
