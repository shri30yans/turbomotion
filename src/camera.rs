//! Camera maths: the keyed zoom and pan of photo shots.

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Where the camera looks: normalised image centre and zoom. Held in f64: the lateral-drift test below sits exactly on a parameter value
/// (a 0.02 pan) that single precision rounds to the other side of.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub cx: f64,
    pub cy: f64,
    pub zoom: f64,
}

/// Everything the photo shader needs to place one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub rect: Rect,
    pub shift: [f32; 2],
    pub dolly: f32,
    pub focus: [f32; 2],
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Smoothstep blended with a linear ramp, so moves start gently but never stall.
pub fn ease_cam(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t) * 0.35 + t * 0.65
}

/// Cover-fit an image of aspect `ar` in a `width`×`height` frame, zoomed and centred on the view.
pub fn cover_rect(ar: f32, width: f32, height: f32, view: View) -> Rect {
    let (ar, width, height) = (f64::from(ar), f64::from(width), f64::from(height));
    let (bw, bh) = if ar > width / height { (height * ar, height) } else { (width, width / ar) };
    let (w, h) = (bw * view.zoom, bh * view.zoom);
    Rect {
        x: (width / 2.0 - view.cx * w).clamp(width - w, 0.0) as f32,
        y: (height / 2.0 - view.cy * h).clamp(height - h, 0.0) as f32,
        w: w as f32,
        h: h as f32,
    }
}

/// Parse a `"x,y,zoom"` shot parameter.
pub fn parse_view(spec: &str) -> Result<View> {
    let mut parts = spec.split(',').map(|p| p.trim().parse::<f64>());
    let mut next = || parts.next().context("view needs x,y,zoom")?.context("view component is not a number");
    let view = View { cx: next()?, cy: next()?, zoom: next()? };
    if parts.next().is_some() {
        bail!("view {spec:?} has more than three components");
    }
    Ok(view)
}

/// Depth-parallax room: the image is oversized so displaced edge pixels stay inside it.
const DEPTH_MARGIN: f32 = 1.09;
const LATERAL_AMPLITUDE: f32 = 0.016;
const DOLLY_AMPLITUDE: f32 = 0.07;

pub struct Move {
    pub from: View,
    pub to: View,
    pub duration: f32,
    pub has_depth: bool,
}

impl Move {
    /// Placement at `seconds` into the shot.
    pub fn at(&self, seconds: f32, aspect: f32, width: f32, height: f32) -> Placement {
        let t = (f64::from(seconds) / f64::from(self.duration).max(0.5)).min(1.0);
        let k = ease_cam(t);
        let margin = if self.has_depth { f64::from(DEPTH_MARGIN) } else { 1.0 };
        let view = View {
            cx: lerp(self.from.cx, self.to.cx, k),
            cy: lerp(self.from.cy, self.to.cy, k),
            zoom: lerp(self.from.zoom, self.to.zoom, k) * margin,
        };
        let (dx, dy) = (self.to.cx - self.from.cx, self.to.cy - self.from.cy);
        let len = dx.hypot(dy);
        let lateral = if len > 0.02 { [dx / len, dy / len] } else { [1.0, 0.0] };
        let (amp, dolly) = if self.has_depth {
            let dz = self.to.zoom - self.from.zoom;
            (f64::from(LATERAL_AMPLITUDE), if dz == 0.0 { 0.0001 } else { dz }.signum() * f64::from(DOLLY_AMPLITUDE) * k)
        } else {
            (0.0, 0.0)
        };
        let drift = (k - 0.5) * 2.0;
        Placement {
            rect: cover_rect(aspect, width, height, view),
            shift: [(lateral[0] * amp * drift) as f32, (lateral[1] * amp * drift * 0.6) as f32],
            dolly: dolly as f32,
            focus: [view.cx as f32, view.cy as f32],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ease_is_monotonic_and_pinned() {
        assert_eq!(ease_cam(0.0), 0.0);
        assert!((ease_cam(1.0) - 1.0).abs() < 1e-6);
        assert!((0..100).all(|i| ease_cam(f64::from(i) / 100.0) <= ease_cam(f64::from(i + 1) / 100.0)));
    }

    #[test]
    fn cover_fills_frame_and_clamps_to_edges() {
        let r = cover_rect(1.0, 1920.0, 1080.0, View { cx: 0.0, cy: 0.0, zoom: 1.0 });
        assert_eq!((r.x, r.y, r.w, r.h), (0.0, 0.0, 1920.0, 1920.0));
        let r = cover_rect(1.0, 1920.0, 1080.0, View { cx: 1.0, cy: 1.0, zoom: 1.0 });
        assert_eq!((r.x, r.y), (0.0, 1080.0 - 1920.0));
    }

    #[test]
    fn a_pan_of_exactly_two_hundredths_still_drifts_along_the_pan() {
        // 0.5 - 0.52 is just over 0.02 in double precision, just under it in single: the drift
        // must follow the vertical pan (JavaScript's answer), not the default sideways one.
        let from = parse_view("0.5,0.52,1.14").unwrap();
        let to = parse_view("0.5,0.5,1.0").unwrap();
        let placement = Move { from, to, duration: 4.467, has_depth: true }.at(1.0, 1.5, 1920.0, 1080.0);
        assert_eq!(placement.shift[0], 0.0);
        assert!(placement.shift[1] != 0.0);
    }

    #[test]
    fn view_parsing_rejects_malformed_specs() {
        assert!(parse_view("0.5,0.5,1.2").is_ok());
        assert!(parse_view("0.5,0.5").is_err());
        assert!(parse_view("a,b,c").is_err());
        assert!(parse_view("1,2,3,4").is_err());
    }
}
