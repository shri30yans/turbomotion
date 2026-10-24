//! Glitch cut: rows of the frame slip sideways and the red and blue channels part, strongest mid-transition.

use std::f64::consts::PI;

use anyhow::Result;
use skia_safe::{Canvas, Image};

use super::compose::{run, texels};
use crate::studio::{
    model::{H, W},
    rng::RandomState,
    shader::Uniforms,
    stage::Stage,
};

const SEED: u32 = 3;
/// Slices `glitch.sksl` can take: the most the reference ever draws (`14 * 1.0 + 2`).
const MAX_SLICES: usize = 16;

pub fn glitch(st: &Stage, like: &Canvas, a: &Image, b: &Image, u: f64) -> Result<Image> {
    // the slices are random but fixed per twelfth of the transition, drawn from numpy's generator like the reference
    let mut rng = RandomState::new(SEED + (u * 12.0) as u32);
    let base = if u < 0.5 { a } else { b };
    let amp = (PI * u).sin();
    let rows = i64::from(H);
    let slices: Vec<[f64; 4]> = (0..(14.0 * amp) as usize + 2)
        .map(|_| {
            let first = rng.randint(0, rows - 20);
            let height = rng.randint(6, 90);
            let shift = (rng.randn() * 120.0 * amp).trunc();
            [first as f64, (first + height).min(rows) as f64, shift, 0.0]
        })
        .collect();
    let count = slices.len();
    let mut table = slices;
    table.resize(MAX_SLICES, [0.0; 4]);
    let shader = st.shaders.transitions.glitch.instance(
        Uniforms::new().f(f64::from(W)).f((26.0 * amp).trunc()).f(count as f64).rows4(&table),
        &[texels(base)?],
    )?;
    run(like, shader)
}
