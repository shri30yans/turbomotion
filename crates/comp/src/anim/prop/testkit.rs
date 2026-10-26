//! Helpers the property tests share: parse from JSON, sample, and an independent reference for After Effects' maths.

use anyhow::Result;
use serde_json::Value;

use super::{Prop, VecProp};
use crate::anim::expr::{Arg, Env};

pub struct NoEnv;

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

pub fn scalar(j: Value) -> Prop {
    Prop::parse(&j).unwrap_or_else(|e| panic!("{e:#}"))
}

pub fn scalar_err(j: Value) -> String {
    format!("{:#}", Prop::parse(&j).expect_err("should be refused"))
}

pub fn position(j: Value) -> VecProp {
    VecProp::parse_position(&j, 2..=3).unwrap_or_else(|e| panic!("{e:#}"))
}

pub fn position_err(j: Value) -> String {
    format!("{:#}", VecProp::parse_position(&j, 2..=3).expect_err("should be refused"))
}

pub fn at(p: &VecProp, t: f64) -> Vec<f64> {
    p.at(t, &NoEnv).unwrap_or_else(|e| panic!("{e:#}"))
}

pub fn dist(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum::<f64>().sqrt()
}

/// After Effects' temporal bezier, written out from its definition with plain bisection: the value at fraction `x` of
/// a segment of `dur` seconds from `a` to `b`, leaving at `(speed_out, influence_out)` and arriving at
/// `(speed_in, influence_in)`.
pub fn reference(a: f64, b: f64, dur: f64, out: (f64, f64), into: (f64, f64), x: f64) -> f64 {
    let cubic = |p: [f64; 4], u: f64| (1.0 - u).powi(3) * p[0] + 3.0 * (1.0 - u).powi(2) * u * p[1] + 3.0 * (1.0 - u) * u * u * p[2] + u.powi(3) * p[3];
    let xs = [0.0, out.1, 1.0 - into.1, 1.0];
    let ys = [a, a + out.0 * out.1 * dur, b - into.0 * into.1 * dur, b];
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..200 {
        let mid = (lo + hi) / 2.0;
        if cubic(xs, mid) < x { lo = mid } else { hi = mid }
    }
    cubic(ys, (lo + hi) / 2.0)
}
