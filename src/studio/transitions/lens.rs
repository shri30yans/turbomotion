//! Transitions that fly the camera through the cut: a zoom-through and a spin, both with radial motion blur.

use std::f64::consts::PI;

use anyhow::Result;
use skia_safe::{Canvas, Image, TileMode};

use super::compose::{crossfade, flash, run, sampled, warped};
use crate::studio::{
    ease::{expo_io, smooth},
    geom::Mat3,
    model::{H, W},
    shader::Uniforms,
    stage::Stage,
};

/// Below this amount the reference skips the blur altogether.
const MIN_BLUR: f64 = 0.002;

/// Radial zoom blur about the frame centre averaging `taps` scaled copies (`fx.zoom_blur`).
fn zoom_blur(st: &Stage, like: &Canvas, image: &Image, amount: f64, taps: u32) -> Result<Image> {
    if amount <= MIN_BLUR {
        return Ok(image.clone());
    }
    let shader = st.shaders.transitions.zoom_blur.instance(Uniforms::new().v2(f64::from(W), f64::from(H)).f(amount).f(f64::from(taps)), &[sampled(image, TileMode::Clamp)?])?;
    run(like, shader)
}

fn scale_about_centre(s: f64) -> Mat3 {
    Mat3([[s, 0.0, f64::from(W) / 2.0 * (1.0 - s)], [0.0, s, f64::from(H) / 2.0 * (1.0 - s)], [0.0, 0.0, 1.0]])
}

/// Zoom-through: A rushes up and out with blur while B settles in from larger, with a flash at the hinge.
pub fn zoom(st: &Stage, like: &Canvas, a: &Image, b: &Image, u: f64) -> Result<Image> {
    let k = expo_io(u);
    let lens = |image: &Image, scale: f64, amount: f64| zoom_blur(st, like, &warped(like, image, &scale_about_centre(scale))?, amount, 8);
    let out_a = lens(a, 1.0 + 1.2 * k, 0.18 * k)?;
    let in_b = lens(b, 1.35 - 0.35 * k, 0.18 * (1.0 - k))?;
    let mixed = crossfade(like, &out_a, &in_b, smooth((u - 0.30) / 0.4))?;
    flash(like, &mixed, (-((u - 0.5) / 0.12).powi(2)).exp() * 0.22, &st.shaders.add_clip.blend(Uniforms::new())?)
}

/// Spin: A turns a quarter away while growing, B turns in from a quarter the other way, both blurred.
pub fn spin(st: &Stage, like: &Canvas, a: &Image, b: &Image, u: f64) -> Result<Image> {
    let k = expo_io(u);
    let (cx, cy) = (f64::from(W) / 2.0, f64::from(H) / 2.0);
    let amount = 0.08 * (PI * u).sin() + 0.01;
    let turned = |image: &Image, angle: f64, scale: f64| zoom_blur(st, like, &warped(like, image, &Mat3::rotation_about(cx, cy, angle, scale))?, amount, 6);
    let out_a = turned(a, -90.0 * k, 1.0 + 0.8 * k)?;
    let in_b = turned(b, 90.0 * (1.0 - k), 1.8 - 0.8 * k)?;
    crossfade(like, &out_a, &in_b, smooth((u - 0.35) / 0.3))
}
