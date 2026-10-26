//! How a value moves between two keys in time: After Effects' temporal interpolation.
//!
//! Each side of a key has a speed (units per second) and an influence (how far across the segment's time the handle
//! reaches). On the graph of value against time they are the two inner control points of one cubic Bezier from key to
//! key: the leaving handle ends at `(influence_out * dur, value + speed_out * influence_out * dur)` and the arriving one
//! at `(dur - influence_in * dur, value_next - speed_in * influence_in * dur)`. Both influences are kept as given, even
//! when they overlap.

use anyhow::{bail, Result};

use super::{
    key::{Handle, Key, Side},
    value::Kind,
};
use crate::anim::{bezier, ease::Ease};

/// The influence of a linear side: a handle a third of the way across the segment at the segment's own speed draws a
/// straight line.
pub const LINEAR_INFLUENCE: f64 = 1.0 / 3.0;

/// The influence After Effects gives an Auto Bezier side (16.667 %).
pub const AUTO_INFLUENCE: f64 = 1.0 / 6.0;

/// How one segment between two keys is shaped.
#[derive(Debug, Clone)]
pub(super) enum Shape {
    /// The value stays at the first key until the second.
    Hold,
    /// A named ease: the same progress for every dimension, and along a motion path.
    Named(Ease),
    /// Speed and influence on each side; `None` is a linear side (the segment's own speed, a third of the way).
    Handles { out: Option<Handle>, into: Option<Handle> },
}

fn side(h: &Option<Handle>, dim: usize, linear_speed: f64) -> (f64, f64) {
    match h {
        None => (linear_speed, LINEAR_INFLUENCE),
        Some(h) => (h.speed.get(dim).or(h.speed.first()).copied().unwrap_or(linear_speed), h.influence),
    }
}

impl Shape {
    /// Dimension `dim` of the value at fraction `x` of a segment of `dur` seconds that runs from `a` to `b`. Along a
    /// motion path the dimension is the distance travelled, from 0 to the length of the path.
    pub fn value(&self, dim: usize, a: f64, b: f64, dur: f64, x: f64) -> f64 {
        match self {
            Shape::Hold => a,
            Shape::Named(e) => a + (b - a) * e.at(x),
            Shape::Handles { out, into } => {
                let linear = (b - a) / dur;
                let ((so, io), (si, ii)) = (side(out, dim, linear), side(into, dim, linear));
                let u = bezier::solve(io, 1.0 - ii, x);
                bezier::point([a, a + so * io * dur, b - si * ii * dur, b], u)
            }
        }
    }

    /// The shape of the segment from key `i` to key `i + 1`.
    pub fn between(keys: &[Key], i: usize, kind: Kind, along_path: bool) -> Result<Shape> {
        let (a, b) = (&keys[i], &keys[i + 1]);
        if !kind.interpolates() {
            return Ok(Shape::Hold);
        }
        let (out, into) = (a.ease_out.as_ref(), b.ease_in.as_ref());
        match (out, into, b.ease) {
            (None, None, ease) => Ok(Shape::Named(ease.unwrap_or_default())),
            (_, _, Some(ease)) => bail!("the move into key {} has the ease {ease:?} and also ease_out on key {i} or ease_in on key {}: a segment takes a named ease or speed and influence, not both", i + 1, i + 1),
            (out, into, None) => {
                let (out, into) = (out.unwrap_or(&Side::Linear), into.unwrap_or(&Side::Linear));
                match (out, into) {
                    (Side::Hold, _) => Ok(Shape::Hold),
                    (_, Side::Hold) => bail!("key {} arrives with ease_in \"hold\", but only the side that leaves a key can hold: write \"hold\" as ease_out on key {i}", i + 1),
                    (Side::Linear, Side::Linear) => Ok(Shape::Named(Ease::Linear)),
                    (o, n) => Ok(Shape::Handles { out: handle(keys, i, o, along_path)?, into: handle(keys, i + 1, n, along_path)? }),
                }
            }
        }
    }
}

/// The handle a side stands for: none for a linear one, the neighbours' slope for an auto one.
fn handle(keys: &[Key], i: usize, side: &Side, along_path: bool) -> Result<Option<Handle>> {
    Ok(match side {
        Side::Linear => None,
        Side::Bezier(h) => Some(h.clone()),
        Side::Auto => Some(Handle { speed: auto_speed(keys, i, along_path), influence: AUTO_INFLUENCE }),
        Side::Hold => bail!("a hold has no handle"),
    })
}

/// The speed of an Auto Bezier key: the slope of the straight line from the key before to the key after (per
/// dimension, or along a motion path the distance between them). The first and last key have no such line and stand
/// still.
fn auto_speed(keys: &[Key], i: usize, along_path: bool) -> Vec<f64> {
    if i == 0 || i + 1 == keys.len() {
        return vec![0.0];
    }
    let (before, after) = (keys[i - 1].v.parts(), keys[i + 1].v.parts());
    let dt = (keys[i + 1].t - keys[i - 1].t).max(1e-9);
    let slope: Vec<f64> = (0..before.n).map(|d| (after.v[d] - before.v[d]) / dt).collect();
    if along_path { vec![slope.iter().map(|s| s * s).sum::<f64>().sqrt()] } else { slope }
}
