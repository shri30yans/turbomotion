//! Entrances and exits for a whole layer: named presets that turn a progress (0 = hidden, 1 = shown) into small
//! changes of opacity, position, scale, rotation and blur. The presets follow the measured motion rules: arrivals land
//! on a long tail, exits are shorter and accelerate, a whip carries horizontal blur on its fastest frames.

use anyhow::{bail, Result};

use crate::{anim::ease::Ease, schema::Transit};

/// What a transition does to a layer at one instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mod {
    pub opacity: f64,
    pub dx: f64,
    pub dy: f64,
    pub scale: f64,
    pub rotation: f64,
    /// Gaussian blur sigma in pixels.
    pub blur: f64,
    /// Horizontal-only blur distance for whips.
    pub smear: f64,
}

impl Mod {
    pub const NONE: Mod = Mod { opacity: 1.0, dx: 0.0, dy: 0.0, scale: 1.0, rotation: 0.0, blur: 0.0, smear: 0.0 };

    pub fn combine(self, o: Mod) -> Mod {
        Mod {
            opacity: self.opacity * o.opacity,
            dx: self.dx + o.dx,
            dy: self.dy + o.dy,
            scale: self.scale * o.scale,
            rotation: self.rotation + o.rotation,
            blur: self.blur.max(o.blur),
            smear: self.smear.max(o.smear),
        }
    }
}

pub const PRESETS: &[(&str, &str)] = &[
    ("fade", "opacity only"),
    ("slide_up", "rises into place from `dist` px below"),
    ("slide_down", "drops into place from `dist` px above"),
    ("slide_left", "slides in from the right"),
    ("slide_right", "slides in from the left"),
    ("pop", "scales up from 60 % with a small overshoot"),
    ("zoom_in", "settles from 130 % to 100 %"),
    ("zoom_out", "settles from 70 % to 100 %"),
    ("blur", "resolves from a blur of `blur` px"),
    ("whip_left", "a fast horizontal whip with motion smear, from the right"),
    ("whip_right", "a fast horizontal whip with motion smear, from the left"),
    ("spin", "turns 90 degrees into place while fading"),
    ("drop", "falls in from above and bounces"),
];

/// Progress 0..1 of `shown` (already eased by the caller) → modifiers. `entering` picks the default distances.
pub fn apply(t: &Transit, shown: f64) -> Result<Mod> {
    let hidden = 1.0 - shown;
    let dist = t.dist.unwrap_or(120.0);
    let blur = t.blur.unwrap_or(14.0);
    let mut m = Mod::NONE;
    match t.preset.as_str() {
        "fade" => m.opacity = shown.clamp(0.0, 1.0),
        "slide_up" => {
            m.opacity = shown.clamp(0.0, 1.0);
            m.dy = dist * hidden;
        }
        "slide_down" => {
            m.opacity = shown.clamp(0.0, 1.0);
            m.dy = -dist * hidden;
        }
        "slide_left" => {
            m.opacity = shown.clamp(0.0, 1.0);
            m.dx = dist * hidden;
        }
        "slide_right" => {
            m.opacity = shown.clamp(0.0, 1.0);
            m.dx = -dist * hidden;
        }
        "pop" => {
            m.opacity = (shown * 4.0).clamp(0.0, 1.0);
            m.scale = 1.0 - 0.4 * hidden;
        }
        "zoom_in" => {
            m.opacity = shown.clamp(0.0, 1.0);
            m.scale = 1.0 + 0.3 * hidden;
        }
        "zoom_out" => {
            m.opacity = shown.clamp(0.0, 1.0);
            m.scale = 1.0 - 0.3 * hidden;
        }
        "blur" => {
            m.opacity = shown.clamp(0.0, 1.0);
            m.blur = blur * hidden.max(0.0);
        }
        "whip_left" | "whip_right" => {
            let sign = if t.preset == "whip_left" { 1.0 } else { -1.0 };
            let d = t.dist.unwrap_or(900.0);
            m.opacity = (shown * 3.0).clamp(0.0, 1.0);
            m.dx = sign * d * hidden;
            // the smear peaks where the layer moves fastest: the ease's steepest part is early, so use the derivative of hidden
            m.smear = (blur * 3.0) * (4.0 * hidden * shown.max(0.0)).min(1.0).max(hidden * 0.5);
        }
        "spin" => {
            m.opacity = shown.clamp(0.0, 1.0);
            m.rotation = -90.0 * hidden;
            m.scale = 1.0 - 0.2 * hidden;
        }
        "drop" => {
            m.opacity = (shown * 6.0).clamp(0.0, 1.0);
            m.dy = -dist * 2.0 * hidden;
        }
        other => bail!("unknown enter/exit preset {other:?}; presets are {}", PRESETS.iter().map(|p| p.0).collect::<Vec<_>>().join(", ")),
    }
    Ok(m)
}

/// The default ease of an entrance (long tail) and of an exit (accelerates away); presets override it with `ease`.
pub fn ease_of(t: &Transit, entering: bool) -> Result<Ease> {
    match &t.ease {
        Some(e) => Ease::parse(e),
        None if t.preset == "pop" || t.preset == "drop" => Ok(if entering { Ease::parse("back.out(1.5)")? } else { Ease::parse("power3.in")? }),
        None => Ok(if entering { Ease::parse("expo.out")? } else { Ease::parse("power3.in")? }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_is_neutral_when_shown_and_valid_when_hidden() {
        for (name, _) in PRESETS {
            let t = Transit { preset: (*name).into(), ..Transit::default() };
            assert_eq!(apply(&t, 1.0).unwrap(), Mod::NONE, "{name}");
            let h = apply(&t, 0.0).unwrap();
            assert!(h.opacity < 1.0 || h.dx != 0.0 || h.dy != 0.0 || h.scale != 1.0 || h.blur > 0.0, "{name} hides nothing");
        }
        assert!(apply(&Transit { preset: "wobble".into(), ..Transit::default() }, 0.5).unwrap_err().to_string().contains("slide_up"));
    }
}
