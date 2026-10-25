//! Shot transitions: presets that move a whole shot. Direction names the way the picture travels; the same name
//! on the outgoing shot's `exit` and the incoming shot's `enter` moves both as one strip.

use super::{Clip, Mod};
use crate::schema::Transit;

/// How much bigger the outgoing shot is than the incoming one all through a `cross_zoom`. Both zoom at the same
/// exponential rate, so the ratio never changes and nothing speeds up or slows down where one replaces the other.
const CROSS_ZOOM_RATIO: f64 = 2.0;
/// A `window`'s portal when the author gives none: centred, a third of the frame wide.
const DEFAULT_PORTAL: [f64; 3] = [0.5, 0.5, 0.34];

/// Presets that move a whole shot; their default ease is symmetric so the two sides of a transition stay locked.
pub fn is_shot_preset(name: &str) -> bool {
    name.starts_with("push_") || name.starts_with("whip_pan_") || name.starts_with("pan_out_") || matches!(name, "zoom_through" | "zoom_back" | "cross_zoom" | "window" | "blur_dissolve" | "glitch" | "recede")
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let u = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// Travel direction of a directional shot preset as a unit vector, and the default distance along it: one frame.
fn heading(name: &str, frame: [f64; 2]) -> ([f64; 2], f64) {
    if name.ends_with("_left") {
        ([-1.0, 0.0], frame[0])
    } else if name.ends_with("_right") {
        ([1.0, 0.0], frame[0])
    } else if name.ends_with("_up") {
        ([0.0, -1.0], frame[1])
    } else {
        ([0.0, 1.0], frame[1])
    }
}

/// Shot transitions. `h` is how hidden the shot is (0 on screen, 1 gone); both sides of one transition see
/// `h_out = e` and `h_in = 1 - e` for the same eased progress `e`, so every curve here is symmetric about 0.5.
pub fn apply(t: &Transit, h: f64, entering: bool, frame: [f64; 2]) -> Mod {
    let name = t.preset.as_str();
    let (dir, d0) = heading(name, frame);
    let d = t.dist.unwrap_or(d0).abs();
    // an entering shot waits ahead of the travel (+), an exiting one has gone behind it (-)
    let side = if entering { -1.0 } else { 1.0 };
    let blur = t.blur.unwrap_or(14.0);
    let mut m = Mod::NONE;
    match name {
        _ if name.starts_with("push_") => {
            m.dx = side * dir[0] * d * h;
            m.dy = side * dir[1] * d * h;
        }
        _ if name.starts_with("whip_pan_") => {
            m.dx = side * dir[0] * d * h;
            m.dy = side * dir[1] * d * h;
            // speed of a symmetric ease peaks at the cut; the smear follows it
            let speed = (4.0 * h * (1.0 - h)).clamp(0.0, 1.0);
            m.smear = blur * 9.0 * speed;
            m.blur = blur * 0.4 * speed;
        }
        _ if name.starts_with("pan_out_") => {
            // pull back over the first third, travel through the middle, push in over the last third
            let back = smoothstep(0.0, 0.32, h.min(1.0 - h));
            let s = 1.0 - 0.44 * back;
            let travel = smoothstep(0.3, 0.7, h);
            let gap = 0.07 * d;
            m.scale = s;
            m.dx = side * dir[0] * (d * s + gap) * travel;
            m.dy = side * dir[1] * (d * s + gap) * travel;
            m.smear = blur * 1.2 * (4.0 * travel * (1.0 - travel));
        }
        "zoom_through" => {
            if entering {
                m.scale = 1.0 - 0.45 * h;
                m.opacity = 1.0 - smoothstep(0.45, 0.85, h);
                m.blur = blur * h;
            } else {
                // stays opaque: the incoming shot above it covers it as it arrives
                m.scale = 1.0 + 2.6 * h * h;
                m.blur = blur * 1.6 * h;
            }
        }
        "zoom_back" => {
            if entering {
                m.scale = 1.0 + 2.2 * h * h;
                m.opacity = 1.0 - smoothstep(0.45, 0.85, h);
                m.blur = blur * 1.6 * h;
            } else {
                m.scale = 1.0 - 0.4 * h;
                m.blur = blur * h;
            }
        }
        "cross_zoom" => {
            // p: how far the cut has gone (0 = outgoing alone, 1 = incoming alone)
            let p = if entering { 1.0 - h } else { h };
            m.scale = CROSS_ZOOM_RATIO.powf(if entering { p - 1.0 } else { p });
            m.opacity = if entering { smoothstep(0.1, 0.7, p) } else { 1.0 };
            m.blur = blur * 0.9 * (4.0 * p * (1.0 - p));
        }
        "window" => {
            let [cx, cy, sr] = t.portal.unwrap_or(DEFAULT_PORTAL);
            let sr = sr.clamp(0.05, 0.95);
            let radius = t.radius.unwrap_or(0.026 * frame[0].min(frame[1])).max(0.0);
            // p: 0 = the portal is a small card, 1 = it has opened to the whole frame
            let p = if entering { 1.0 - h } else { h };
            // portal centre relative to the frame centre, and the portal's width as a fraction of the frame
            let c = [(cx - 0.5) * frame[0], (cy - 0.5) * frame[1]];
            let portal_w = sr.powf(1.0 - p);
            if entering {
                // the next shot is the card: shrunk to the portal, riding it from its place to the centre
                m.scale = portal_w;
                m.dx = c[0] * (1.0 - p);
                m.dy = c[1] * (1.0 - p);
                m.clip = (p < 1.0 - 1e-12).then_some(Clip::Round { rect: [0.0, 0.0, 1.0, 1.0], radius: radius * (1.0 - p) });
            } else {
                // the last shot zooms about the portal so the portal grows to the frame, and slides it to the centre
                let s = portal_w / sr;
                m.scale = s;
                m.dx = c[0] * (1.0 - s - p);
                m.dy = c[1] * (1.0 - s - p);
            }
            m.blur = blur * 0.35 * (4.0 * p * (1.0 - p));
        }
        "blur_dissolve" => {
            // the outgoing shot only blurs; the incoming one, above it, fades in
            m.opacity = if entering { 1.0 - h } else { 1.0 };
            m.blur = blur * 2.0 * (4.0 * h * (1.0 - h)).max(h * 0.5);
        }
        "glitch" => {
            // deterministic jitter: a hash of the progress bucket, never of wall time
            let k = (h * 23.0).floor();
            let j = ((k * 12.9898).sin() * 43758.5453).fract() * 2.0 - 1.0;
            let live = 4.0 * h * (1.0 - h);
            m.dx = j * 70.0 * live;
            m.smear = blur * 5.0 * live * j.abs();
            m.opacity = if h > 0.5 { if j > 0.2 { 0.0 } else { 1.0 - h } } else { 1.0 };
        }
        "recede" => {
            m.scale = 1.0 - 0.1 * h;
            m.opacity = 1.0 - 0.65 * h;
            m.blur = blur * 0.5 * h;
        }
        _ => {}
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: [f64; 2] = [1920.0, 1080.0];

    fn pair(name: &str, e: f64) -> (Mod, Mod) {
        let t = Transit { preset: name.into(), ..Transit::default() };
        (apply(&t, e, false, FRAME), apply(&t, 1.0 - e, true, FRAME))
    }

    /// A cross zoom is one continuous dolly: the incoming shot is always exactly `CROSS_ZOOM_RATIO` times smaller
    /// than the outgoing one, so the zoom rate is the same on both sides of every instant.
    #[test]
    fn cross_zoom_keeps_a_constant_ratio() {
        for i in 0..=40 {
            let (out, inn) = pair("cross_zoom", f64::from(i) / 40.0);
            assert!((out.scale / inn.scale - CROSS_ZOOM_RATIO).abs() < 1e-9, "step {i}: {} / {}", out.scale, inn.scale);
        }
    }

    /// A window's incoming card and the portal it opens in the outgoing shot are the same rectangle at every
    /// instant: same size (the outgoing portal is `sr * out.scale` wide) and same centre.
    #[test]
    fn window_card_matches_the_portal_it_opens() {
        let portal = [0.7, 0.35, 0.3];
        let t_out = Transit { preset: "window".into(), portal: Some(portal), ..Transit::default() };
        for i in 0..=40 {
            let e = f64::from(i) / 40.0;
            let out = apply(&t_out, e, false, FRAME);
            let inn = apply(&t_out, 1.0 - e, true, FRAME);
            // portal width on screen as the outgoing shot sees it, against the card's own width
            assert!((portal[2] * out.scale - inn.scale).abs() < 1e-9, "width at {e}");
            // portal centre on screen: the outgoing shot's portal point is moved by scale about the frame centre plus (dx, dy)
            let rel = [(portal[0] - 0.5) * FRAME[0], (portal[1] - 0.5) * FRAME[1]];
            let out_centre = [rel[0] * out.scale + out.dx, rel[1] * out.scale + out.dy];
            assert!((out_centre[0] - inn.dx).abs() < 1e-6 && (out_centre[1] - inn.dy).abs() < 1e-6, "centre at {e}: {out_centre:?} vs ({}, {})", inn.dx, inn.dy);
        }
    }

    /// The card's corners square off as it opens and it ends with no clip at all.
    #[test]
    fn window_opens_to_a_square_frame() {
        let t = Transit { preset: "window".into(), radius: Some(40.0), ..Transit::default() };
        match apply(&t, 1.0, true, FRAME).clip {
            Some(Clip::Round { radius, .. }) => assert!((radius - 40.0).abs() < 1e-9),
            other => panic!("{other:?}"),
        }
        assert!(apply(&t, 0.0, true, FRAME).clip.is_none());
    }

    /// Both sides of a push or pan move as one strip: the gap between the two shot centres never changes sign
    /// and the outgoing shot is always behind the incoming one along the travel.
    #[test]
    fn shot_pairs_travel_together() {
        for name in ["push_left", "whip_pan_right", "pan_out_up", "pan_out_left"] {
            let t = Transit { preset: name.into(), ..Transit::default() };
            let (dir, _) = heading(name, FRAME);
            for i in 0..=20 {
                let e = f64::from(i) / 20.0;
                let out = apply(&t, e, false, FRAME);
                let inn = apply(&t, 1.0 - e, true, FRAME);
                let along = (inn.dx - out.dx) * -dir[0] + (inn.dy - out.dy) * -dir[1];
                assert!(along >= 0.0 && (out.scale - inn.scale).abs() < 1e-9, "{name} at {e}: {along}");
            }
        }
    }

}
