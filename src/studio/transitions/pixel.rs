//! Pixelate: the frame is averaged into ever larger blocks and back, peaking mid-transition.

use std::f64::consts::PI;

use anyhow::Result;
use skia_safe::{Canvas, Image};

use super::compose::{run, run_sized, texels};
use crate::studio::{
    model::{H, W},
    shader::Uniforms,
    stage::Stage,
};

pub fn pixelate(st: &Stage, like: &Canvas, a: &Image, b: &Image, u: f64) -> Result<Image> {
    let base = if u < 0.5 { a } else { b };
    let block = (1.0 + (PI * u).sin() * 46.0) as i32;
    let (small_w, small_h) = ((W / block).max(2), (H / block).max(2));
    let programs = &st.shaders.transitions;
    let across = run_sized(like, small_w, H, programs.area.instance(Uniforms::new().v2(1.0, 0.0).f(f64::from(W) / f64::from(small_w)), &[texels(base)?])?)?;
    let small = run_sized(like, small_w, small_h, programs.area.instance(Uniforms::new().v2(0.0, 1.0).f(f64::from(H) / f64::from(small_h)), &[texels(&across)?])?)?;
    run(like, programs.pixel_up.instance(Uniforms::new().v2(f64::from(small_w) / f64::from(W), f64::from(small_h) / f64::from(H)), &[texels(&small)?])?)
}
