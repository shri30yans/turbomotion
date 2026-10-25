//! Speed ramps: a speed curve (source seconds per output second) integrated into a time-remap curve.

use anyhow::{ensure, Result};
use serde_json::{json, Value};

use super::round;
use crate::anim::{
    expr::{Arg, Env},
    prop::Prop,
};

struct NoEnv;

impl Env for NoEnv {
    fn var(&self, _: &str) -> Option<f64> {
        None
    }
    fn call(&self, _: &str, _: &[Arg]) -> Option<Result<f64>> {
        None
    }
    fn salt(&self) -> u32 {
        0
    }
}

/// Source time as the integral of the speed curve, sampled per frame (linear between frames is exact enough at
/// any frame rate the renderer samples).
pub fn integrate(speed: &Prop, dur: f64, fps: f64, offset: f64) -> Result<(Value, f64)> {
    let sub = 8.0;
    let dt = 1.0 / (fps * sub);
    let frames = (dur * fps).ceil() as usize;
    let mut keys = vec![json!([0, round(offset), "none"])];
    let (mut t, mut acc) = (0.0, offset);
    let mut prev = speed.at(0.0, &NoEnv)?;
    for f in 1..=frames {
        let target = (f as f64 / fps).min(dur);
        while t < target - 1e-12 {
            let t1 = (t + dt).min(target);
            let s1 = speed.at(t1, &NoEnv)?;
            ensure!(prev >= 0.0 && s1 >= 0.0, "speed must not be negative (reverse playback is a remap, not a speed)");
            acc += (prev + s1) * 0.5 * (t1 - t);
            prev = s1;
            t = t1;
        }
        keys.push(json!([round(target), round(acc), "none"]));
    }
    Ok((Value::Array(keys), acc))
}
