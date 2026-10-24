//! Transitions that reveal B across the frame by a moving front: whip, slide, radial, diagonal wipe, ripple and the
//! light leak.

use std::f64::consts::PI;

use anyhow::Result;
use skia_safe::{Canvas, Image, TileMode};

use super::{
    compose::{run, sampled},
    Heading,
};
use crate::{{model::{H, W}, stage::{Stage}}};
use turbomotion_core::{ease::{expo_io, pow_io, pow_out}, shader::{Uniforms}};

fn frame_size() -> Uniforms {
    Uniforms::new().v2(f64::from(W), f64::from(H))
}

/// Whip pan: the frames slide across together under a horizontal motion blur that peaks mid-move.
pub fn whip(st: &Stage, like: &Canvas, a: &Image, b: &Image, u: f64, heading: Heading) -> Result<Image> {
    const TAPS: f64 = 12.0;
    let direction = heading.sign();
    let width = f64::from(W);
    let offset = expo_io(u) * width * direction;
    let amount = (PI * u).sin() * width * 0.20;
    let cut = (width - offset.abs()).round_ties_even().max(0.0);
    let shader = st.shaders.transitions.whip.instance(
        Uniforms::new().f(width).f(amount).f(offset).f(direction).f(cut).f(TAPS),
        &[sampled(a, TileMode::Clamp)?, sampled(b, TileMode::Clamp)?],
    )?;
    run(like, shader)
}

/// Parallax slide: B pushes in over A while A drifts back and dims.
pub fn slide(st: &Stage, like: &Canvas, a: &Image, b: &Image, u: f64, heading: Heading) -> Result<Image> {
    let direction = heading.sign();
    let progress = pow_io(u, 3.0);
    let offset = ((1.0 - progress) * f64::from(W) * direction).trunc();
    let shader = st.shaders.transitions.slide.instance(
        Uniforms::new().f(f64::from(W)).f(progress).f(direction).f(offset),
        &[sampled(a, TileMode::Clamp)?, sampled(b, TileMode::Decal)?],
    )?;
    run(like, shader)
}

/// Circular reveal from the centre with a glowing rim.
pub fn radial(st: &Stage, like: &Canvas, a: &Image, b: &Image, u: f64) -> Result<Image> {
    const SOFT: f64 = 0.08;
    let shader = st.shaders.transitions.radial.instance(
        frame_size().f(expo_io(u) * (1.0 + SOFT * 2.0)).f(SOFT),
        &[sampled(a, TileMode::Clamp)?, sampled(b, TileMode::Clamp)?],
    )?;
    run(like, shader)
}

/// Soft diagonal wipe with a bright edge.
pub fn luma(st: &Stage, like: &Canvas, a: &Image, b: &Image, u: f64) -> Result<Image> {
    const ANGLE: f64 = 25.0;
    const SOFT: f64 = 0.18;
    let (sin, cos) = f64::to_radians(ANGLE).sin_cos();
    let extent = (f64::from(W) - 1.0) * cos + (f64::from(H) - 1.0) * sin;
    let front = pow_io(u, 3.0) * (1.0 + SOFT * 2.0) - SOFT;
    let shader = st.shaders.transitions.luma.instance(
        Uniforms::new().v2(cos, sin).f(extent).f(front).f(SOFT),
        &[sampled(a, TileMode::Clamp)?, sampled(b, TileMode::Clamp)?],
    )?;
    run(like, shader)
}

/// A shockwave that ripples outward and dissolves A into B behind it.
pub fn ripple(st: &Stage, like: &Canvas, a: &Image, b: &Image, u: f64) -> Result<Image> {
    let front = pow_out(u, 2.0) * f64::from(W).hypot(f64::from(H)) / 2.0 * 1.1;
    let shader = st.shaders.transitions.ripple.instance(
        frame_size().f(front).f(40.0 * (PI * u).sin()),
        &[sampled(a, TileMode::Clamp)?, sampled(b, TileMode::Clamp)?],
    )?;
    run(like, shader)
}

/// A warm light leak flaring across the cut; the frame underneath changes at the midpoint.
pub fn leak(st: &Stage, like: &Canvas, a: &Image, b: &Image, u: f64) -> Result<Image> {
    let base = if u < 0.5 { a } else { b };
    let shader = st.shaders.transitions.leak.instance(frame_size().f((PI * u).sin()).f(-0.2 + 1.4 * u), &[sampled(base, TileMode::Clamp)?])?;
    run(like, shader)
}
