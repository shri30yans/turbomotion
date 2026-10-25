//! Entrances and exits for a whole layer: named presets that turn a progress (0 = hidden, 1 = shown) into small
//! changes of opacity, position, scale, rotation and blur. The presets follow the measured motion rules: arrivals land
//! on a long tail, exits are shorter and accelerate, a whip carries horizontal blur on its fastest frames.

use anyhow::{bail, Result};
use serde::Serialize;

use crate::{anim::ease::Ease, schema::Transit};

mod shot;
pub use shot::is_shot_preset;

/// A clip in the layer's own 0..1 box. `None` on `Mod` means fully shown.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Clip {
    /// Keep `x0,y0,x1,y1` of the layer box (GSAP clipPath inset).
    Rect([f64; 4]),
    /// As `Rect`, with rounded corners of `radius` screen pixels.
    Round { rect: [f64; 4], radius: f64 },
    /// Circle at the box centre; `1` covers the box, `0` is a point.
    Circle(f64),
}

impl Clip {
    pub fn reveals(self) -> bool {
        match self {
            Clip::Rect(r) | Clip::Round { rect: r, .. } => (r[2] - r[0]).max(0.0) * (r[3] - r[1]).max(0.0) > 0.01,
            Clip::Circle(k) => k > 0.02,
        }
    }

    fn intersect(self, o: Clip) -> Clip {
        match (self, o) {
            (Clip::Rect(a), Clip::Rect(b)) => Clip::Rect([a[0].max(b[0]), a[1].max(b[1]), a[2].min(b[2]), a[3].min(b[3])]),
            (Clip::Circle(a), Clip::Circle(b)) => Clip::Circle(a.min(b)),
            (r @ (Clip::Rect(_) | Clip::Round { .. }), _) | (_, r @ (Clip::Rect(_) | Clip::Round { .. })) => r,
        }
    }
}

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
    pub clip: Option<Clip>,
}

impl Mod {
    pub const NONE: Mod = Mod { opacity: 1.0, dx: 0.0, dy: 0.0, scale: 1.0, rotation: 0.0, blur: 0.0, smear: 0.0, clip: None };

    pub fn combine(self, o: Mod) -> Mod {
        Mod {
            opacity: self.opacity * o.opacity,
            dx: self.dx + o.dx,
            dy: self.dy + o.dy,
            scale: self.scale * o.scale,
            rotation: self.rotation + o.rotation,
            blur: self.blur.max(o.blur),
            smear: self.smear.max(o.smear),
            clip: match (self.clip, o.clip) {
                (None, x) | (x, None) => x,
                (Some(a), Some(b)) => Some(a.intersect(b)),
            },
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
    ("stamp", "lands oversized and rotated, like a rubber stamp"),
    ("toss", "flies off on a short arc, rotating (GSAP Inertia)"),
    ("wipe_left", "a clip opens from the left (GSAP clipPath)"),
    ("wipe_right", "a clip opens from the right"),
    ("wipe_up", "a clip opens from the bottom"),
    ("wipe_down", "a clip opens from the top"),
    ("iris", "a circle opens from the centre"),
    ("tumble", "a fast 3D-feeling spin: rotates, overshoots scale, smears"),
    // Shot transitions. Direction names the way the picture travels; the same name on the outgoing shot's `exit`
    // and the incoming shot's `enter` moves both as one strip. `dist` defaults to a 1920 x 1080 frame.
    ("push_left", "shot transition: the whole frame travels left; enter comes from the right, exit leaves to the left"),
    ("push_right", "shot transition: the frame travels right"),
    ("push_up", "shot transition: the frame travels up"),
    ("push_down", "shot transition: the frame travels down"),
    ("whip_pan_left", "shot transition: a camera whip pan to the left, motion smear peaking mid-move"),
    ("whip_pan_right", "shot transition: a camera whip pan to the right"),
    ("pan_out_left", "shot transition: the camera pulls back until both shots are tiles on one canvas, pans left, pushes into the next"),
    ("pan_out_right", "shot transition: pull back, pan right, push in"),
    ("pan_out_up", "shot transition: pull back, pan up, push in"),
    ("pan_out_down", "shot transition: pull back, pan down, push in"),
    ("zoom_through", "shot transition: fly forward through the outgoing shot into the next (exit grows past the lens, enter grows to fill)"),
    ("zoom_back", "shot transition: fly backward (exit shrinks away, enter arrives from in front of the lens)"),
    ("cross_zoom", "shot transition: a constant-speed dolly; both shots zoom at one rate, so there is no jump in speed where one hands over to the other"),
    ("window", "shot transition: the next shot is a rounded card (`portal`, `radius`) that grows to fill the frame while the last zooms toward it"),
    ("blur_dissolve", "shot transition: a dissolve through a soft blur"),
    ("glitch", "shot transition: jittering slices of horizontal smear and a hard flicker"),
    ("recede", "shot transition, outgoing only: steps back and dims while the next shot slides over it"),
];

/// Progress 0..1 of `shown` (already eased by the caller) → modifiers. `entering` tells shot presets which side of
/// the cut the layer is on: an entering shot sits ahead of the travel, an exiting one behind it.
pub fn apply(t: &Transit, shown: f64, entering: bool, frame: [f64; 2]) -> Result<Mod> {
    let hidden = 1.0 - shown;
    if is_shot_preset(&t.preset) {
        return Ok(shot::apply(t, hidden.clamp(0.0, 1.0), entering, frame));
    }
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
        "stamp" => {
            m.opacity = (shown * 10.0).clamp(0.0, 1.0);
            m.scale = 1.0 + 0.38 * hidden;
            m.rotation = -9.0 * hidden;
        }
        "toss" => {
            m.opacity = shown.clamp(0.0, 1.0);
            m.dx = dist * hidden;
            m.dy = dist * 0.4 * hidden * hidden;
            m.rotation = 16.0 * hidden;
        }
        "wipe_left" => m.clip = (shown < 1.0 - 1e-12).then_some(Clip::Rect([0.0, 0.0, shown.max(0.0), 1.0])),
        "wipe_right" => m.clip = (shown < 1.0 - 1e-12).then_some(Clip::Rect([1.0 - shown.max(0.0), 0.0, 1.0, 1.0])),
        "wipe_up" => m.clip = (shown < 1.0 - 1e-12).then_some(Clip::Rect([0.0, 1.0 - shown.max(0.0), 1.0, 1.0])),
        "wipe_down" => m.clip = (shown < 1.0 - 1e-12).then_some(Clip::Rect([0.0, 0.0, 1.0, shown.max(0.0)])),
        "iris" => m.clip = (shown < 1.0 - 1e-12).then_some(Clip::Circle(shown.max(0.0))),
        "tumble" => {
            let d = t.dist.unwrap_or(240.0);
            m.opacity = (shown * 5.0).clamp(0.0, 1.0);
            m.dx = d * hidden;
            m.rotation = -110.0 * hidden;
            m.scale = 1.0 + 0.22 * hidden;
            m.smear = (blur * 2.4) * (4.0 * hidden * shown.max(0.0)).min(1.0);
        }
        other => bail!("unknown enter/exit preset {other:?}; presets are {}", PRESETS.iter().map(|p| p.0).collect::<Vec<_>>().join(", ")),
    }
    Ok(m)
}

/// The default ease of an entrance (long tail) and of an exit (accelerates away); presets override it with `ease`.
pub fn ease_of(t: &Transit, entering: bool) -> Result<Ease> {
    match &t.ease {
        Some(e) => Ease::parse(e),
        None if matches!(t.preset.as_str(), "cross_zoom" | "window") => Ease::parse("power3.inOut"),
        None if is_shot_preset(&t.preset) => Ease::parse("sine.inOut"),
        None if t.preset == "pop" || t.preset == "drop" || t.preset == "stamp" => Ok(if entering { Ease::parse("back.out(1.5)")? } else { Ease::parse("power3.in")? }),
        None => Ok(if entering { Ease::parse("expo.out")? } else { Ease::parse("power3.in")? }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: [f64; 2] = [1920.0, 1080.0];

    #[test]
    fn every_preset_is_neutral_when_shown_and_valid_when_hidden() {
        for (name, _) in PRESETS {
            let t = Transit { preset: (*name).into(), ..Transit::default() };
            assert_eq!(apply(&t, 1.0, true, FRAME).unwrap(), Mod::NONE, "{name}");
            assert_eq!(apply(&t, 1.0, false, FRAME).unwrap(), Mod::NONE, "{name}");
            let h = apply(&t, 0.0, !name.starts_with("recede"), FRAME).unwrap();
            assert!(h.opacity < 1.0 || h.dx != 0.0 || h.dy != 0.0 || h.scale != 1.0 || h.blur > 0.0 || h.clip.is_some(), "{name} hides nothing");
        }
        assert!(apply(&Transit { preset: "wobble".into(), ..Transit::default() }, 0.5, true, FRAME).unwrap_err().to_string().contains("slide_up"));
    }
}
