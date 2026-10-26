//! Effects: a library of layer effects that any element or whole scene can carry as an `fx` list.
//!
//! ```json
//! {"k": "title", "text": "HELLO", "fx": [{"effect": "glow", "radius": 30}, {"effect": "blur", "radius": [[2, 24], [3, 0]]}]}
//! ```
//!
//! Each effect takes the layer built so far and returns a new one, in list order. Numbers may be animated with keys
//! `[[seconds, value], ...]`. `turbomotion studio effects` prints the catalogue; the registry below is its only source,
//! so the documentation cannot drift from what the renderer accepts.

mod backdrop;
mod blend;
mod blur;
mod color;
mod distort;
mod edge;
mod param;
mod pass;
mod place;
mod programs;
mod spec;
mod texture;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::{Map, Value};
use skia_safe::Canvas;

pub use backdrop::over as backdrop_over;
pub use blend::Blend;
pub use blur::smear;
pub use pass::Ctx;
pub use programs::Programs;
pub use spec::{check_names, ParamKind, ParamSpec};

use self::spec::{float, optional_rgb, resolve_all, rgb};
use super::{grade, layer::Layer, shaders::Shaders};

/// One catalogue entry: what the effect is called, what it does, every parameter it takes, and a runnable use.
pub struct Entry {
    pub effect: &'static str,
    pub summary: &'static str,
    pub params: &'static [ParamSpec],
    /// A use of the effect worth showing, with values chosen to be visible (the defaults are often neutral).
    pub example: fn() -> Value,
}

macro_rules! registry {
    ($( $variant:ident($ty:ty) = $name:literal, $func:path, $summary:literal, [$($param:expr),* $(,)?], $example:tt; )*) => {
        /// An effect and its parameters. Make one with [`Effect::build`], which checks every parameter against the
        /// declarations below and fills in the defaults.
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

        /// Every effect, in catalogue order. This is the only declaration of what a platform effect takes.
        pub static CATALOGUE: &[Entry] = &[
            $( Entry { effect: $name, summary: $summary, params: &[$($param),*], example: || serde_json::json!($example) }, )*
        ];
    };
}

registry! {
    Blur(blur::Blur) = "blur", blur::blur, "Gaussian blur.",
        [float("radius", 8.0, "Gaussian sigma in pixels.").at_least(0.0)],
        {"effect": "blur", "radius": 8};
    Backdrop(backdrop::Backdrop) = "backdrop", backdrop::backdrop, "Blurs what has already been composited behind this layer and draws the layer on top (frosted glass).",
        [float("blur", 24.0, "Gaussian sigma in pixels.").at_least(0.0)],
        {"effect": "backdrop", "blur": 24};
    DirectionalBlur(blur::DirectionalBlur) = "directional_blur", blur::directional_blur, "Motion blur: smears the layer along an angle.",
        [float("distance", 40.0, "Length of the smear in pixels."), float("angle", 0.0, "Direction of the smear in degrees (0 = horizontal, 90 = vertical).")],
        {"effect": "directional_blur", "distance": 40, "angle": 0};
    RadialBlur(blur::RadialBlur) = "radial_blur", blur::radial_blur, "Zoom blur from a point.",
        [float("amount", 0.1, "Extra zoom at the end of the smear: 0.1 smears 10%."), float("x", 0.5, "Centre across the frame, as a fraction."), float("y", 0.5, "Centre down the frame, as a fraction.")],
        {"effect": "radial_blur", "amount": 0.1, "x": 0.5, "y": 0.5};
    Glow(blur::Glow) = "glow", blur::glow, "Adds a blurred copy back on top, so bright parts bleed light.",
        [
            float("radius", 24.0, "Blur sigma of the halo in pixels.").at_least(0.0),
            float("strength", 1.0, "How much of the halo is added back (1 = as bright as the source).").at_least(0.0),
            float("threshold", 0.0, "Only parts brighter than this (0..1 luma) glow; 0 makes everything glow.").between(0.0, 1.0),
            optional_rgb("color", "Colour of the halo; leave out to glow in the layer's own colours."),
        ],
        {"effect": "glow", "radius": 24, "strength": 1, "threshold": 0, "color": [255, 200, 120]};
    Shadow(edge::Shadow) = "shadow", edge::shadow, "Soft drop shadow behind the layer's shape.",
        [
            float("dx", 0.0, "Horizontal offset in pixels."),
            float("dy", 12.0, "Vertical offset in pixels."),
            float("blur", 16.0, "Gaussian sigma of the shadow's edge.").at_least(0.0),
            rgb("color", [0, 0, 0], "Shadow colour."),
            float("opacity", 0.5, "How dark the shadow is.").between(0.0, 1.0),
        ],
        {"effect": "shadow", "dx": 0, "dy": 12, "blur": 16, "color": [0, 0, 0], "opacity": 0.5};
    Outline(edge::Outline) = "outline", edge::outline, "Solid outline around the layer's shape.",
        [
            float("width", 4.0, "Thickness in pixels.").at_least(0.0),
            rgb("color", [255, 255, 255], "Outline colour."),
            float("opacity", 1.0, "How solid the outline is.").between(0.0, 1.0),
        ],
        {"effect": "outline", "width": 4, "color": [255, 255, 255], "opacity": 1};
    Brightness(color::Brightness) = "brightness", color::brightness, "Adds a fixed amount to every channel.",
        [float("amount", 0.0, "Added to every channel (0..1 scale): 0.2 is a clear lift, negative darkens.")],
        {"effect": "brightness", "amount": 0.2};
    Contrast(color::Contrast) = "contrast", color::contrast, "Spreads or squeezes the tones around mid-grey.",
        [float("amount", 1.0, "1 leaves the layer alone, 0 is flat grey, 2 is punchy.")],
        {"effect": "contrast", "amount": 1.3};
    Saturation(color::Saturation) = "saturation", color::saturation, "Strengthens or drains colour.",
        [float("amount", 1.0, "1 leaves the layer alone, 0 is black and white, 2 is vivid.")],
        {"effect": "saturation", "amount": 1.4};
    Hue(color::Hue) = "hue", color::hue, "Rotates colours around the wheel.",
        [float("degrees", 0.0, "Degrees around the colour wheel.")],
        {"effect": "hue", "degrees": 90};
    Exposure(color::Exposure) = "exposure", color::exposure, "Photographic stops of light.",
        [float("amount", 0.0, "Photographic stops: +1 doubles the light, -1 halves it.")],
        {"effect": "exposure", "amount": 0.5};
    Gamma(color::Gamma) = "gamma", color::gamma, "Mid-tone curve.",
        [float("amount", 1.0, "Above 1 lightens the mid-tones, below 1 darkens them.")],
        {"effect": "gamma", "amount": 1.2};
    Invert(color::Invert) = "invert", color::invert, "Negative.",
        [float("amount", 1.0, "How far to the negative: 1 fully inverts, 0.5 is flat grey.")],
        {"effect": "invert", "amount": 1};
    Tint(color::Tint) = "tint", color::tint, "Paints the layer toward a colour, keeping its shape.",
        [
            rgb("color", [255, 255, 255], "The colour the layer is painted toward."),
            float("amount", 1.0, "0 keeps the layer's colours, 1 paints it all in `color` (the alpha shape stays).").between(0.0, 1.0),
        ],
        {"effect": "tint", "color": [255, 120, 60], "amount": 0.6};
    Grade(grade::Grade) = "grade", color::grade, "The scene colour grade on one layer.",
        [
            float("lift", grade::NEUTRAL_LIFT, "Added to the darks."),
            float("gamma", grade::NEUTRAL_GAMMA, "Mid-tone curve."),
            float("gain", grade::NEUTRAL_GAIN, "Multiplies the lights."),
            float("sat", grade::NEUTRAL_SAT, "Colour strength: 1 unchanged, 0 black and white."),
            float("contrast", grade::NEUTRAL_CONTRAST, "Spread around mid-grey."),
            optional_rgb("tint", "Colour the picture is pushed toward; leave out for no tint."),
            float("tint_k", grade::NEUTRAL_TINT_K, "How far toward `tint` (0..1); ignored without a tint."),
        ],
        {"effect": "grade", "lift": 0, "gamma": 1, "gain": 1, "sat": 0.8, "contrast": 1.1, "tint": [20, 40, 80], "tint_k": 0.1};
    Wave(distort::Wave) = "wave", distort::wave, "Sine wave displacement.",
        [
            float("amplitude", 12.0, "How far pixels move, in pixels."),
            float("wavelength", 160.0, "Distance between crests in pixels.").at_least(1.0),
            float("speed", 0.5, "Crests travelling past per second (negative reverses)."),
            float("angle", 0.0, "Direction the wave travels in degrees (0 = left to right); pixels move across it."),
        ],
        {"effect": "wave", "amplitude": 12, "wavelength": 160, "speed": 0.5, "angle": 0};
    Twirl(distort::Twirl) = "twirl", distort::twirl, "Swirls the picture around a point.",
        [
            float("angle", 90.0, "Turn at the centre in degrees (it eases to nothing at the radius)."),
            float("radius", 300.0, "Reach in pixels.").at_least(1.0),
            float("x", 0.5, "Centre across the frame, as a fraction."),
            float("y", 0.5, "Centre down the frame, as a fraction."),
        ],
        {"effect": "twirl", "angle": 90, "radius": 300, "x": 0.5, "y": 0.5};
    Bulge(distort::Bulge) = "bulge", distort::bulge, "Magnifies or pinches the picture around a point.",
        [
            float("strength", 0.5, "0.5 magnifies the centre, negative pinches it in."),
            float("radius", 300.0, "Reach in pixels.").at_least(1.0),
            float("x", 0.5, "Centre across the frame, as a fraction."),
            float("y", 0.5, "Centre down the frame, as a fraction."),
        ],
        {"effect": "bulge", "strength": 0.5, "radius": 300, "x": 0.5, "y": 0.5};
    Pixelate(distort::Pixelate) = "pixelate", distort::pixelate, "Mosaic of square blocks.",
        [float("size", 16.0, "Block size in pixels.").at_least(1.0)],
        {"effect": "pixelate", "size": 16};
    ChromaticAberration(distort::ChromaticAberration) = "chromatic_aberration", distort::chromatic_aberration, "Red and blue part ways toward the corners, like a cheap lens.",
        [float("amount", 6.0, "Red/blue separation in pixels at the frame corners (zero at the centre).")],
        {"effect": "chromatic_aberration", "amount": 6};
    RgbSplit(distort::RgbSplit) = "rgb_split", distort::rgb_split, "Red and blue shift apart along an angle; green stays.",
        [float("distance", 8.0, "Red and blue part ways by this many pixels."), float("angle", 0.0, "Direction in degrees (0 = horizontal).")],
        {"effect": "rgb_split", "distance": 8, "angle": 0};
    Grain(texture::Grain) = "grain", texture::grain, "Film grain, redrawn 24 times a second.",
        [float("amount", 0.12, "Strength of the noise (0.1 is visible, 0.3 is heavy)."), float("seed", 0.0, "Pattern seed.").at_least(0.0)],
        {"effect": "grain", "amount": 0.12, "seed": 0};
    Scanlines(texture::Scanlines) = "scanlines", texture::scanlines, "CRT scanlines.",
        [
            float("spacing", 4.0, "Distance between lines in pixels.").at_least(2.0),
            float("strength", 0.35, "How much the dark part of each line dims the picture (0..1)."),
            float("thickness", 0.5, "Fraction of each spacing that is dark.").between(0.05, 0.85),
        ],
        {"effect": "scanlines", "spacing": 4, "strength": 0.35, "thickness": 0.5};
    Vignette(texture::Vignette) = "vignette", texture::vignette, "Darkens toward the corners.",
        [
            float("strength", 0.5, "0 = none, 1 = black corners."),
            float("softness", 0.45, "How far from the centre the falloff starts, as a fraction of the half diagonal (0..1)."),
        ],
        {"effect": "vignette", "strength": 0.5, "softness": 0.45};
    Opacity(place::Opacity) = "opacity", place::opacity, "Fades the whole layer.",
        [float("value", 1.0, "0 = invisible, 1 = unchanged.").between(0.0, 1.0)],
        {"effect": "opacity", "value": 0.6};
    Transform(place::Transform) = "transform", place::transform, "Moves, scales and rotates the layer about an anchor.",
        [
            float("x", 0.0, "Move right by this many pixels."),
            float("y", 0.0, "Move down by this many pixels."),
            float("scale", 1.0, "Uniform scale (1 = unchanged)."),
            float("rotation", 0.0, "Clockwise degrees."),
            float("anchor_x", 0.5, "The point scale and rotation pivot about, across the frame as a fraction."),
            float("anchor_y", 0.5, "The same point, down the frame."),
        ],
        {"effect": "transform", "x": 40, "y": -20, "scale": 1.15, "rotation": -6, "anchor_x": 0.5, "anchor_y": 0.5};
}

impl Effect {
    /// The effect `name` with `given` parameters: unknown names and parameters are errors that list the valid ones, a
    /// parameter left out takes its declared default, and values are held inside their declared range.
    pub fn build(name: &str, given: &Map<String, Value>) -> Result<Effect> {
        let entry = entry(name).ok_or_else(|| anyhow!("no platform effect {name:?}; they are: {}", CATALOGUE.iter().map(|e| e.effect).collect::<Vec<_>>().join(", ")))?;
        let resolved = resolve_all(name, entry.params, given)?;
        serde_json::from_value(Value::Object(resolved)).with_context(|| format!("effect {name:?}"))
    }
}

/// The catalogue entry called `name`.
pub fn entry(name: &str) -> Option<&'static Entry> {
    CATALOGUE.iter().find(|e| e.effect == name)
}

/// Run `chain` over `layer` at time `t`, in order.
pub fn apply(shaders: &Shaders, like: &Canvas, layer: Layer, chain: &[Effect], t: f64) -> Result<Layer> {
    let ctx = Ctx { shaders, like, t };
    chain.iter().try_fold(layer, |layer, effect| effect.run(&ctx, &layer).with_context(|| format!("effect {:?}", effect.name())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params_of(v: &Value) -> Map<String, Value> {
        let mut o = v.as_object().expect("an example is an object").clone();
        o.remove("effect");
        o
    }

    #[test]
    fn every_catalogue_example_builds_as_its_own_effect_with_declared_parameters_only() {
        assert!(CATALOGUE.len() >= 25);
        for entry in CATALOGUE {
            let example = (entry.example)();
            assert_eq!(example["effect"], entry.effect, "{}", entry.effect);
            let effect = Effect::build(entry.effect, &params_of(&example)).unwrap_or_else(|e| panic!("{}: {e:#}", entry.effect));
            assert_eq!(effect.name(), entry.effect);
        }
    }

    #[test]
    fn leaving_every_parameter_out_builds_every_effect_from_its_defaults() {
        for entry in CATALOGUE {
            Effect::build(entry.effect, &Map::new()).unwrap_or_else(|e| panic!("{}: {e:#}", entry.effect));
        }
    }

    #[test]
    fn names_are_unique_and_parameter_names_are_unique_within_an_effect() {
        let mut names: Vec<_> = CATALOGUE.iter().map(|e| e.effect).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
        for entry in CATALOGUE {
            let mut params: Vec<_> = entry.params.iter().map(|p| p.name).collect();
            params.sort_unstable();
            let before = params.len();
            params.dedup();
            assert_eq!(before, params.len(), "{}", entry.effect);
        }
    }

    #[test]
    fn every_parameter_is_documented_and_has_a_sane_range() {
        for entry in CATALOGUE {
            assert!(!entry.summary.is_empty(), "{}", entry.effect);
            for p in entry.params {
                assert!(!p.doc.is_empty(), "{} {}", entry.effect, p.name);
                if let (Some(lo), Some(hi)) = (p.min, p.max) {
                    assert!(lo < hi, "{} {}", entry.effect, p.name);
                }
            }
        }
    }

    #[test]
    fn unknown_effects_and_parameters_are_errors() {
        let none = Map::new();
        assert!(Effect::build("sparkle", &none).unwrap_err().to_string().contains("blur"));
        let e = Effect::build("blur", &params_of(&serde_json::json!({"radis": 3}))).unwrap_err().to_string();
        assert!(e.contains("\"radis\"") && e.contains("radius"), "{e}");
        let keys_out_of_order = params_of(&serde_json::json!({"radius": [[1, 2], [0, 3]]}));
        assert!(Effect::build("blur", &keys_out_of_order).unwrap_err().to_string().contains("parameter \"radius\""));
    }

    #[test]
    fn a_value_outside_the_declared_range_is_held_at_its_edge() {
        let Effect::Pixelate(p) = Effect::build("pixelate", &params_of(&serde_json::json!({"size": 0.2}))).unwrap() else { panic!("a pixelate") };
        assert_eq!(p.size.at(0.0), 1.0);
    }
}
