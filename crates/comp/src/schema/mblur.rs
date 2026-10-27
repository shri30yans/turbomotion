//! Motion blur: the shutter that decides which moments a blurred frame is averaged over.
//!
//! Two keys of a composition end as one [`Shutter`]. `"motion_blur"` is After Effects' own model: a shutter angle, a
//! shutter phase and a number of samples. `"mblur"` is the older key, whose shutter is always centred on the frame
//! time; it is kept as it was so that films made with it render as they did.

use anyhow::{bail, ensure, Result};
use serde::Deserialize;

/// Most sub-frames a frame is averaged over. Each one draws every blurred layer again, so a frame costs this many times
/// as much as the same frame without blur.
pub const MAX_BLUR_SAMPLES: u32 = 16;

/// The shutter angle's range in degrees (After Effects: 0 to 720).
pub const ANGLE_RANGE: (f64, f64) = (0.0, 720.0);

/// The shutter phase's range in degrees (After Effects: -360 to 360).
pub const PHASE_RANGE: (f64, f64) = (-360.0, 360.0);

/// The older key `"mblur"`: `{"samples": 8, "shutter": 180}`. The shutter is centred on the frame time.
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(default, deny_unknown_fields)]
pub struct MotionBlur {
    /// Sub-frames averaged per frame.
    pub samples: u32,
    /// Shutter angle in degrees (180 = half the frame interval).
    pub shutter: f64,
}

impl Default for MotionBlur {
    fn default() -> Self {
        MotionBlur { samples: 8, shutter: 180.0 }
    }
}

impl MotionBlur {
    /// The same blur as a shutter: open for `shutter` degrees, starting that much before the frame time's middle.
    pub fn shutter(self) -> Shutter {
        Shutter { shutter_angle: self.shutter, shutter_phase: -self.shutter / 2.0, samples: self.samples }
    }
}

/// How the shutter of a composition is open: the key `"motion_blur"`, after After Effects' Composition Settings.
///
/// The frame at time `t` shows the scene averaged over `[t + phase, t + phase + angle]`, both turned from degrees of a
/// frame (360 degrees = one frame interval) into seconds. The defaults, 180 and -90, are After Effects': a shutter open
/// for half the frame, centred on the frame time. Layers blur only when they say `"mblur": true`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Shutter {
    /// How long the shutter is open, in degrees of the frame interval, 0 to 720 (default 180). 0 draws no blur.
    pub shutter_angle: f64,
    /// When the shutter opens relative to the frame time, in degrees of the frame interval, -360 to 360 (default -90, which
    /// centres a 180 degree shutter on the frame time; 0 opens it at the frame time).
    pub shutter_phase: f64,
    /// Sub-frames averaged per frame, 2 to 16 (default 16). Every sample draws each blurred layer again.
    pub samples: u32,
}

impl Default for Shutter {
    fn default() -> Self {
        Shutter { shutter_angle: 180.0, shutter_phase: -90.0, samples: MAX_BLUR_SAMPLES }
    }
}

impl Shutter {
    /// The block as written: its numbers must lie in the ranges After Effects allows, and the samples in `2..=16`.
    pub fn checked(self) -> Result<Shutter> {
        let (a, p) = (ANGLE_RANGE, PHASE_RANGE);
        ensure!(self.shutter_angle.is_finite() && (a.0..=a.1).contains(&self.shutter_angle), "motion_blur.shutter_angle is {}, but must be between {} and {} degrees", self.shutter_angle, a.0, a.1);
        ensure!(self.shutter_phase.is_finite() && (p.0..=p.1).contains(&self.shutter_phase), "motion_blur.shutter_phase is {}, but must be between {} and {} degrees", self.shutter_phase, p.0, p.1);
        if self.samples > MAX_BLUR_SAMPLES {
            bail!("motion_blur.samples is {}, but at most {MAX_BLUR_SAMPLES} are allowed: every sample draws each blurred layer again, so a frame costs `samples` times as much", self.samples);
        }
        ensure!(self.samples >= 2, "motion_blur.samples is {}, but at least 2 are needed to average anything", self.samples);
        Ok(self)
    }

    /// Seconds the shutter is open in a frame of `fps`.
    pub fn open(&self, fps: f64) -> f64 {
        self.shutter_angle / 360.0 / fps
    }

    /// The moments the frame at `t` is averaged over, evenly spread over the open shutter (the middle of each of
    /// `samples` equal slices). None when the shutter is shut or there is one moment only: no blur.
    pub fn moments(&self, t: f64, fps: f64) -> Vec<f64> {
        if self.samples < 2 || self.shutter_angle <= 0.0 {
            return vec![];
        }
        let (open, start) = (self.open(fps), self.shutter_phase / self.shutter_angle);
        (0..self.samples).map(|i| t + ((f64::from(i) + 0.5) / f64::from(self.samples) + start) * open).collect()
    }
}

/// Which key asked for the blur, because the two differ in more than the shutter's phase.
///
/// `Legacy` (`"mblur"`) keeps what the key always did: the shutter is centred on the frame time whatever its angle, a layer
/// that is not showing at the frame time takes no part, and nothing is range-checked. `AfterEffects` (`"motion_blur"`)
/// integrates over `[t + phase, t + phase + angle]` and draws every layer that shows at any moment of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Blur {
    Legacy(Shutter),
    AfterEffects(Shutter),
}

impl Blur {
    pub fn shutter(&self) -> &Shutter {
        match self {
            Blur::Legacy(s) | Blur::AfterEffects(s) => s,
        }
    }
}

/// What a layer's `"mblur": true` gives a composition that says nothing else: the older key's defaults.
impl Default for Blur {
    fn default() -> Self {
        Blur::Legacy(MotionBlur::default().shutter())
    }
}

/// The blur composition `name` asks for with `"motion_blur"` or the older `"mblur"`; none when it has neither.
pub fn blur_of(name: &str, c: &super::Composition) -> Result<Option<Blur>> {
    let at = if name.is_empty() { "the composition".to_string() } else { format!("composition {name:?}") };
    match (c.mblur, c.motion_blur) {
        (Some(_), Some(_)) => bail!("{at} has both \"mblur\" and \"motion_blur\"; use \"motion_blur\" (shutter_angle, shutter_phase, samples), which is After Effects' model, and drop \"mblur\""),
        (Some(old), None) => Ok(Some(Blur::Legacy(old.shutter()))),
        (None, Some(block)) => block.checked().map(|s| Some(Blur::AfterEffects(s))).map_err(|e| anyhow::anyhow!("{at}: {e}")),
        (None, None) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(v: serde_json::Value) -> Shutter {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn the_defaults_are_after_effects() {
        let s = Shutter::default();
        assert_eq!((s.shutter_angle, s.shutter_phase, s.samples), (180.0, -90.0, 16));
    }

    #[test]
    fn a_180_degree_shutter_at_phase_minus_90_is_centred_and_phase_slides_it_by_its_degrees() {
        let fps = 30.0;
        let spread = |s: Shutter| {
            let m = s.moments(1.0, fps);
            (m[0] - 0.5 * s.open(fps) / f64::from(s.samples), m[m.len() - 1] + 0.5 * s.open(fps) / f64::from(s.samples))
        };
        let (lo, hi) = spread(Shutter::default());
        // 90 degrees = a quarter of the 1/30 s frame, each side of the frame time
        assert!((lo - (1.0 - 0.25 / fps)).abs() < 1e-12 && (hi - (1.0 + 0.25 / fps)).abs() < 1e-12, "{lo} {hi}");
        let (lo, hi) = spread(Shutter { shutter_phase: 0.0, ..Shutter::default() });
        assert!((lo - 1.0).abs() < 1e-12 && (hi - (1.0 + 0.5 / fps)).abs() < 1e-12, "{lo} {hi}");
    }

    #[test]
    fn a_shut_shutter_or_a_single_sample_averages_nothing() {
        assert!(Shutter { shutter_angle: 0.0, ..Shutter::default() }.moments(0.0, 30.0).is_empty());
        assert!(Shutter { samples: 1, ..Shutter::default() }.moments(0.0, 30.0).is_empty());
    }

    #[test]
    fn the_older_key_is_a_shutter_centred_on_the_frame_whatever_its_angle() {
        for angle in [90.0, 180.0, 360.0] {
            let s = MotionBlur { samples: 8, shutter: angle }.shutter();
            let m = s.moments(2.0, 24.0);
            let mid = (m[0] + m[7]) / 2.0;
            assert!((mid - 2.0).abs() < 1e-12, "angle {angle}: centred at {mid}");
        }
    }

    #[test]
    fn numbers_outside_the_ranges_are_refused_with_the_range() {
        let e = |v| block(v).checked().unwrap_err().to_string();
        assert!(e(serde_json::json!({"shutter_angle": 721})).contains("between 0 and 720"));
        assert!(e(serde_json::json!({"shutter_phase": -361})).contains("between -360 and 360"));
        assert!(e(serde_json::json!({"samples": 17})).contains("at most 16") && e(serde_json::json!({"samples": 17})).contains("costs `samples` times"));
        assert!(e(serde_json::json!({"samples": 1})).contains("at least 2"));
        assert!(block(serde_json::json!({"shutter_angle": 720, "shutter_phase": 360, "samples": 16})).checked().is_ok());
    }
}
