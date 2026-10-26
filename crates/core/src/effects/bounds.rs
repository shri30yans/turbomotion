//! How far an effect reaches past the pixels it is given. After Effects grows a layer's buffer by what its effects need
//! (a blur spreads, a drop shadow falls, a displacement pushes pixels out), so the spread is not cut off at the layer's
//! edge. Each platform effect states its own reach here, from its parameters at a moment and the box it works on; a
//! plugin states its reach in its header (`@bounds`). An effect that does not appear in `grows` does not reach past
//! its input.

use super::{
    blur::{DirectionalBlur, Glow, RadialBlur},
    distort::{Bulge, ChromaticAberration, Pixelate, RgbSplit, Twirl, Wave},
    edge::{Outline, Shadow},
    Effect,
};
use crate::geom::Mat3;

/// Pixels added to `[left, top, right, bottom]`; never negative.
pub type Growth = [f64; 4];

/// A Gaussian's reach in standard deviations: past three sigma less than 0.3% of the weight is left.
pub const GAUSSIAN_REACH: f64 = 3.0;

/// The platform effects whose output can reach past their input: exactly those [`Effect::grows`] gives a size.
pub const GROWING: [&str; 13] = ["blur", "directional_blur", "radial_blur", "glow", "shadow", "outline", "wave", "twirl", "bulge", "pixelate", "chromatic_aberration", "rgb_split", "transform"];

/// Whether platform effect `name` can reach past its input (the ledger's `grows_bounds`).
pub fn grows_bounds(name: &str) -> bool {
    GROWING.contains(&name)
}

/// A smear shorter than this leaves the layer alone (the directional blur's own cut-off).
const MIN_SMEAR: f64 = 1.5;

/// `content` grown by `g`.
pub fn grown(content: [f64; 4], g: Growth) -> [f64; 4] {
    [content[0] - g[0], content[1] - g[1], content[2] + g[2], content[3] + g[3]]
}

/// What `after` reaches past `before` on each side.
fn exceeds(before: [f64; 4], after: [f64; 4]) -> Growth {
    [(before[0] - after[0]).max(0.0), (before[1] - after[1]).max(0.0), (after[2] - before[2]).max(0.0), (after[3] - before[3]).max(0.0)]
}

fn all_sides(r: f64) -> Growth {
    [r.max(0.0); 4]
}

/// A disc of `radius` about `centre` meeting the box: how far the disc, cut down to the farthest corner, passes each side.
fn disc_reach(content: [f64; 4], centre: (f64, f64), radius: f64) -> Growth {
    let far = [(content[0], content[1]), (content[2], content[1]), (content[2], content[3]), (content[0], content[3])].iter().map(|c| (c.0 - centre.0).hypot(c.1 - centre.1)).fold(0.0, f64::max);
    let reach = radius.max(0.0).min(far);
    exceeds(content, [centre.0 - reach, centre.1 - reach, centre.0 + reach, centre.1 + reach])
}

impl Effect {
    /// How far the effect reaches past `content` (`[left, top, right, bottom]` in layer pixels) at time `t`.
    pub fn grows(&self, t: f64, content: [f64; 4]) -> Growth {
        let (w, h) = (content[2] - content[0], content[3] - content[1]);
        match self {
            Effect::Blur(e) => all_sides(GAUSSIAN_REACH * e.radius.at(t)),
            Effect::Glow(Glow { radius, .. }) => all_sides(GAUSSIAN_REACH * radius.at(t)),
            Effect::DirectionalBlur(DirectionalBlur { distance, angle }) => {
                let d = distance.at(t);
                if d.abs() < MIN_SMEAR {
                    return [0.0; 4];
                }
                // the copies are spread evenly over `d` pixels centred on the pixel
                let (sin, cos) = angle.at(t).to_radians().sin_cos();
                let (gx, gy) = ((d * cos).abs() / 2.0, (d * sin).abs() / 2.0);
                [gx, gy, gx, gy]
            }
            Effect::RadialBlur(RadialBlur { amount, x, y }) => {
                let a = amount.at(t).max(0.0);
                let (cx, cy) = (content[0] + x.at(t) * w, content[1] + y.at(t) * h);
                // copies are scaled from 1x to (1 + amount)x about the centre: each side moves outward by `amount` of its distance
                [(cx - content[0]).max(0.0) * a, (cy - content[1]).max(0.0) * a, (content[2] - cx).max(0.0) * a, (content[3] - cy).max(0.0) * a]
            }
            Effect::Shadow(Shadow { dx, dy, blur, .. }) => {
                let (dx, dy, reach) = (dx.at(t), dy.at(t), GAUSSIAN_REACH * blur.at(t));
                [(reach - dx).max(0.0), (reach - dy).max(0.0), (reach + dx).max(0.0), (reach + dy).max(0.0)]
            }
            Effect::Outline(Outline { width, .. }) => all_sides(width.at(t)),
            Effect::Wave(Wave { amplitude, angle, .. }) => {
                // pixels move across the direction the wave travels
                let (sin, cos) = angle.at(t).to_radians().sin_cos();
                let a = amplitude.at(t).abs();
                [a * sin.abs(), a * cos.abs(), a * sin.abs(), a * cos.abs()]
            }
            Effect::Twirl(Twirl { radius, x, y, .. }) => disc_reach(content, (content[0] + x.at(t) * w, content[1] + y.at(t) * h), radius.at(t)),
            Effect::Bulge(Bulge { strength, radius, x, y }) if strength.at(t) > 0.0 => disc_reach(content, (content[0] + x.at(t) * w, content[1] + y.at(t) * h), radius.at(t)),
            Effect::Pixelate(Pixelate { size }) => all_sides(size.at(t)),
            Effect::ChromaticAberration(ChromaticAberration { amount }) => all_sides(amount.at(t).abs()),
            Effect::RgbSplit(RgbSplit { distance, angle }) => {
                let (sin, cos) = angle.at(t).to_radians().sin_cos();
                let d = distance.at(t).abs();
                [d * cos.abs(), d * sin.abs(), d * cos.abs(), d * sin.abs()]
            }
            Effect::Transform(e) => {
                let anchor = (content[0] + e.anchor_x.at(t) * w, content[1] + e.anchor_y.at(t) * h);
                let m = Mat3::rotation_about(anchor.0, anchor.1, -e.rotation.at(t), e.scale.at(t)).then_translate(e.x.at(t), e.y.at(t)).0;
                let at = |x: f64, y: f64| (m[0][0] * x + m[0][1] * y + m[0][2], m[1][0] * x + m[1][1] * y + m[1][2]);
                let moved: Vec<(f64, f64)> = [(content[0], content[1]), (content[2], content[1]), (content[2], content[3]), (content[0], content[3])].iter().map(|c| at(c.0, c.1)).collect();
                let fold = |pick: fn(&(f64, f64)) -> f64, f: fn(f64, f64) -> f64, init: f64| moved.iter().map(pick).fold(init, f);
                exceeds(content, [fold(|p| p.0, f64::min, f64::INFINITY), fold(|p| p.1, f64::min, f64::INFINITY), fold(|p| p.0, f64::max, f64::NEG_INFINITY), fold(|p| p.1, f64::max, f64::NEG_INFINITY)])
            }
            _ => [0.0; 4],
        }
    }

    /// Why the effect cannot run on a layer's own pixels, when it cannot: it reads the picture composited behind the
    /// layer, which has no layer-space form.
    pub fn needs_screen(&self) -> Option<&'static str> {
        matches!(self, Effect::Backdrop(_)).then_some("it blurs the picture composited behind the layer, which only exists in screen space")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value};

    use super::*;
    use crate::effects::CATALOGUE;

    const BOX: [f64; 4] = [0.0, 0.0, 100.0, 60.0];

    fn built(name: &str, params: Value) -> Effect {
        let mut o: Map<String, Value> = params.as_object().expect("an object").clone();
        o.remove("effect");
        Effect::build(name, &o).unwrap_or_else(|e| panic!("{name}: {e:#}"))
    }

    fn grows(name: &str, params: Value) -> Growth {
        built(name, params).grows(0.0, BOX)
    }

    #[test]
    fn the_growing_list_is_exactly_the_effects_that_grow_at_their_catalogue_example() {
        for entry in CATALOGUE {
            let g = built(entry.effect, (entry.example)()).grows(0.0, BOX);
            let reaches = g.iter().any(|&v| v > 0.0);
            assert_eq!(reaches, grows_bounds(entry.effect), "{}: grows {g:?} but grows_bounds says {}", entry.effect, grows_bounds(entry.effect));
            assert!(g.iter().all(|&v| v >= 0.0 && v.is_finite()), "{}: {g:?}", entry.effect);
        }
        assert!(GROWING.iter().all(|n| CATALOGUE.iter().any(|e| e.effect == *n)), "every growing name is a real effect");
    }

    #[test]
    fn a_blur_reaches_three_sigma_on_every_side() {
        assert_eq!(grows("blur", serde_json::json!({"radius": 10})), [30.0; 4]);
        assert_eq!(grows("blur", serde_json::json!({"radius": 0})), [0.0; 4]);
    }

    #[test]
    fn a_shadow_reaches_further_on_the_side_it_falls_toward() {
        let g = grows("shadow", serde_json::json!({"dx": 10, "dy": 0, "blur": 5}));
        assert_eq!(g, [5.0, 15.0, 25.0, 15.0], "left, top, right, bottom");
    }

    #[test]
    fn a_directional_blur_reaches_half_its_length_along_its_angle_and_a_glow_like_a_blur() {
        let g = grows("directional_blur", serde_json::json!({"distance": 40, "angle": 0}));
        assert!((g[0] - 20.0).abs() < 1e-9 && g[1].abs() < 1e-9 && (g[2] - 20.0).abs() < 1e-9, "{g:?}");
        let g = grows("directional_blur", serde_json::json!({"distance": 40, "angle": 90}));
        assert!(g[0].abs() < 1e-9 && (g[1] - 20.0).abs() < 1e-9, "{g:?}");
        assert_eq!(grows("glow", serde_json::json!({"radius": 8})), [24.0; 4]);
    }

    #[test]
    fn a_radial_blur_grows_in_proportion_to_the_distance_from_its_centre() {
        let g = grows("radial_blur", serde_json::json!({"amount": 0.2, "x": 0.0, "y": 0.5}));
        assert_eq!(g[0], 0.0);
        assert!((g[2] - 20.0).abs() < 1e-9, "100 px from the centre x 0.2: {g:?}");
        assert!((g[1] - 6.0).abs() < 1e-9 && (g[3] - 6.0).abs() < 1e-9, "{g:?}");
    }

    #[test]
    fn a_transform_grows_by_where_it_carries_the_corners() {
        let g = grows("transform", serde_json::json!({"x": 30, "y": 0, "scale": 1, "rotation": 0}));
        assert!((g[2] - 30.0).abs() < 1e-9 && g[0] == 0.0 && g[1] == 0.0, "{g:?}");
        let g = grows("transform", serde_json::json!({"scale": 2, "anchor_x": 0.0, "anchor_y": 0.0}));
        assert!((g[2] - 100.0).abs() < 1e-9 && (g[3] - 60.0).abs() < 1e-9 && g[0] == 0.0, "{g:?}");
    }

    #[test]
    fn a_distortion_grows_only_by_how_far_its_disc_passes_the_box() {
        assert_eq!(grows("bulge", serde_json::json!({"strength": -0.5, "radius": 50})), [0.0; 4], "a pinch pulls pixels in");
        let far = 50f64.hypot(30.0);
        let g = grows("twirl", serde_json::json!({"radius": 1000, "x": 0.5, "y": 0.5}));
        for (got, want) in g.iter().zip([far - 50.0, far - 30.0, far - 50.0, far - 30.0]) {
            assert!((got - want).abs() < 1e-9, "a twirl turns pixels about its centre, so they stay within the farthest corner's distance: {g:?}");
        }
        assert_eq!(grows("twirl", serde_json::json!({"radius": 10, "x": 0.5, "y": 0.5})), [0.0; 4], "a disc inside the box leaves it alone");
    }

    #[test]
    fn grown_adds_each_side() {
        assert_eq!(grown(BOX, [1.0, 2.0, 3.0, 4.0]), [-1.0, -2.0, 103.0, 64.0]);
    }

    #[test]
    fn backdrop_has_no_layer_space_form() {
        assert!(built("backdrop", serde_json::json!({})).needs_screen().is_some());
        assert!(built("blur", serde_json::json!({})).needs_screen().is_none());
    }
}
