//! Images and video at one instant: which part of the file shows (a source rectangle) and where (a destination
//! rectangle inside the layer's viewport). Cropping, fitting and the Ken Burns move are all decided here, in numbers.

use anyhow::Result;
use serde::Serialize;

use super::{CompPlan, Evaluator, LayerEnv};
use crate::{
    anim::ease::Ease,
    plan::{LayerPlan, Media},
    schema::{Fit, Image, KenBurns, Video},
};

/// What part of a picture fills the viewport.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct View {
    /// Pixels of the file: `[x, y, w, h]`.
    pub src: [f64; 4],
    /// Pixels of the viewport: `[x, y, w, h]`.
    pub dest: [f64; 4],
    pub viewport: [f64; 2],
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct Framing {
    pub zoom: f64,
    pub center: [f64; 2],
}

/// Fit a `iw` x `ih` picture into a `vw` x `vh` viewport, looking at `framing` (zoom 1 = the plain fit).
pub fn fit(fit: Fit, iw: f64, ih: f64, vw: f64, vh: f64, framing: Framing) -> View {
    let z = framing.zoom.max(0.01);
    let viewport = [vw, vh];
    match fit {
        Fit::Cover | Fit::Fill | Fit::None => {
            let (sx, sy) = match fit {
                Fit::Cover => {
                    let s = (vw / iw).max(vh / ih);
                    (s, s)
                }
                Fit::Fill => (vw / iw, vh / ih),
                _ => (1.0, 1.0),
            };
            let (rw, rh) = ((vw / sx / z).min(iw), (vh / sy / z).min(ih));
            let l = (framing.center[0] * iw - rw / 2.0).clamp(0.0, iw - rw);
            let t = (framing.center[1] * ih - rh / 2.0).clamp(0.0, ih - rh);
            // when the region is clamped to the picture the destination shrinks with it, so nothing stretches
            View { src: [l, t, rw, rh], dest: [0.0, 0.0, rw * sx * z.max(1.0).min(vw / (rw * sx)), rh * sy * z.max(1.0).min(vh / (rh * sy))].map(|v| v), viewport }
                .normalised(vw, vh)
        }
        Fit::Contain => {
            let s = (vw / iw).min(vh / ih) * z;
            let (dw, dh) = (iw * s, ih * s);
            // the point of the picture at `center` sits at the viewport centre
            let (cx, cy) = (framing.center[0] * iw * s, framing.center[1] * ih * s);
            View { src: [0.0, 0.0, iw, ih], dest: [vw / 2.0 - cx, vh / 2.0 - cy, dw, dh], viewport }
        }
    }
}

impl View {
    /// Cover-style views fill the whole viewport.
    fn normalised(mut self, vw: f64, vh: f64) -> View {
        self.dest = [0.0, 0.0, vw, vh];
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ParallaxState {
    /// Sampling displacement in viewport pixels at full depth.
    pub shift: [f64; 2],
}

fn framing_of(f: &crate::schema::Framing, focus: [f64; 2]) -> Framing {
    Framing { zoom: f.zoom.unwrap_or(1.0), center: f.center.unwrap_or(focus) }
}

pub fn ken_burns(kb: &Option<KenBurns>, focus: [f64; 2], lt: f64, layer_dur: f64) -> Result<Framing> {
    let Some(kb) = kb else { return Ok(Framing { zoom: 1.0, center: focus }) };
    let (a, b) = (framing_of(&kb.from, focus), framing_of(&kb.to, focus));
    let dur = kb.dur.unwrap_or((layer_dur - kb.at).max(1e-3));
    let u = ((lt - kb.at) / dur.max(1e-6)).clamp(0.0, 1.0);
    let e = kb.ease.as_deref().map_or(Ok(Ease::parse("sine.inOut")?), Ease::parse)?.at(u);
    // zoom interpolates geometrically so the move feels constant-speed
    let zoom = a.zoom * (b.zoom / a.zoom.max(1e-6)).powf(e);
    Ok(Framing { zoom, center: [a.center[0] + (b.center[0] - a.center[0]) * e, a.center[1] + (b.center[1] - a.center[1]) * e] })
}

pub fn image_state(ev: &Evaluator, c: &CompPlan, l: &LayerPlan, im: &Image, t: f64, size: [f64; 2]) -> Result<(View, Option<ParallaxState>)> {
    let lt = t - l.start;
    let Media::Image { info, depth, .. } = &l.media else { unreachable!("image layers are probed at plan time") };
    let focus = im.focus.unwrap_or([0.5, 0.5]);
    let framing = ken_burns(&im.camera, focus, lt, l.end - l.start)?;
    let view = fit(im.fit, f64::from(info.w), f64::from(info.h), size[0], size[1], framing);
    let parallax = match (&im.parallax, depth) {
        (Some(p), Some(_)) => {
            let env = |salt: u32| LayerEnv::new(ev, c, l.index, t, 0).with_salt(salt);
            let strength = p.strength.0.at(lt, &env(super::salt::PARALLAX_STRENGTH))?;
            let look = p.look.as_ref().map_or(Ok(vec![0.0, 0.0]), |v| v.0.at(lt, &env(super::salt::PARALLAX_LOOK)))?;
            let k = strength * size[0];
            Some(ParallaxState { shift: [look[0] * k, look.get(1).copied().unwrap_or(0.0) * k] })
        }
        _ => None,
    };
    Ok((view, parallax))
}

/// The time in the video file shown at layer time `lt`.
pub fn video_time(v: &Video, dur: f64, lt: f64) -> f64 {
    let t = v.src_in + lt * v.speed;
    if v.looping && dur > 0.0 {
        v.src_in + (lt * v.speed).rem_euclid((dur - v.src_in).max(1e-3))
    } else {
        t.clamp(0.0, (dur - 1e-3).max(0.0))
    }
}

pub fn video_view(v: &Video, l: &LayerPlan, size: [f64; 2]) -> View {
    let Media::Video { info, .. } = &l.media else { unreachable!("video layers are probed at plan time") };
    fit(v.fit, f64::from(info.w), f64::from(info.h), size[0], size[1], Framing { zoom: 1.0, center: v.focus.unwrap_or([0.5, 0.5]) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(fit_: Fit, zoom: f64, center: [f64; 2]) -> View {
        fit(fit_, 4000.0, 3000.0, 1920.0, 1080.0, Framing { zoom, center })
    }

    #[test]
    fn cover_crops_to_the_viewport_aspect_and_follows_the_focus() {
        let v = f(Fit::Cover, 1.0, [0.5, 0.5]);
        assert!((v.src[2] / v.src[3] - 1920.0 / 1080.0).abs() < 1e-9);
        assert_eq!(v.src[2], 4000.0);
        let top = f(Fit::Cover, 1.0, [0.5, 0.0]);
        assert_eq!(top.src[1], 0.0);
        let zoomed = f(Fit::Cover, 2.0, [0.5, 0.5]);
        assert!((zoomed.src[2] - 2000.0).abs() < 1e-9);
        assert_eq!(zoomed.dest, [0.0, 0.0, 1920.0, 1080.0]);
    }

    #[test]
    fn a_zoomed_region_never_leaves_the_picture() {
        let v = f(Fit::Cover, 3.0, [0.0, 0.0]);
        assert!(v.src[0] >= 0.0 && v.src[1] >= 0.0 && v.src[0] + v.src[2] <= 4000.0 + 1e-9);
        let v = f(Fit::Cover, 3.0, [1.0, 1.0]);
        assert!(v.src[0] + v.src[2] <= 4000.0 + 1e-9 && v.src[1] + v.src[3] <= 3000.0 + 1e-9);
    }

    #[test]
    fn contain_letterboxes() {
        let v = f(Fit::Contain, 1.0, [0.5, 0.5]);
        assert!((v.dest[2] - 1440.0).abs() < 1e-9 && (v.dest[3] - 1080.0).abs() < 1e-9);
        assert!((v.dest[0] - 240.0).abs() < 1e-9);
    }
}
