//! Two or three numbers moving together: positions, scales, anchors.

use anyhow::{bail, Result};

use super::{
    key::Point,
    track::Track,
    Prop,
};
use crate::anim::expr::Env;

/// Two or three numbers moving together: positions, scales, anchors.
#[derive(Debug, Clone)]
pub enum VecProp {
    /// One property per dimension, each with its own keys and expression: `[x, y]`, or keys of whole vectors that are
    /// split apart.
    Axes(Vec<Prop>),
    /// One track for the whole vector, written as keys with `t`, `v` and handles. A position follows a motion path,
    /// and may turn the layer along it (`auto_orient`).
    Joined { track: Track, auto_orient: bool },
}

impl VecProp {
    pub fn constant(v: &[f64]) -> VecProp {
        VecProp::Axes(v.iter().map(|x| Prop::constant(*x)).collect())
    }

    /// The per-dimension properties of a vector that is written dimension by dimension.
    pub fn into_axes(self) -> Result<Vec<Prop>> {
        match self {
            VecProp::Axes(a) => Ok(a),
            VecProp::Joined { .. } => bail!("this property animates each dimension on its own; write it as [x, y, z] (each part any property) or {{\"separate_dimensions\": [x, y, z]}}"),
        }
    }

    /// The value at `t`, each dimension seeing `env`.
    pub fn at(&self, t: f64, env: &dyn Env) -> Result<Vec<f64>> {
        match self {
            VecProp::Axes(a) => a.iter().map(|p| p.at(t, env)).collect(),
            VecProp::Joined { track, .. } => Ok(track.sample(t).to_vec()),
        }
    }

    /// The value at `t`; the expression of dimension `i` is evaluated in `env(i)` (a whole-vector track has none).
    pub fn at_with<E: Env>(&self, t: f64, env: impl Fn(usize) -> E) -> Result<Vec<f64>> {
        match self {
            VecProp::Axes(a) => a.iter().enumerate().map(|(i, p)| p.at(t, &env(i))).collect(),
            VecProp::Joined { track, .. } => Ok(track.sample(t).to_vec()),
        }
    }

    /// The direction a position travels in at `t` if its layer is to face along the path.
    pub fn heading(&self, t: f64) -> Option<Point> {
        match self {
            VecProp::Joined { track, auto_orient: true } => track.direction(t),
            _ => None,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            VecProp::Axes(a) => a.len(),
            VecProp::Joined { track, .. } => track.kind().dims(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
