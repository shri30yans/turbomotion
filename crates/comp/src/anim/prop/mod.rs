//! Animated properties. Every number a composition animates is a `Prop`; vectors and colours are small bundles of
//! them. A property is any of:
//!
//! ```text
//! 40                                                  a constant
//! "wiggle(2, 30)"                                     an expression (text)
//! [[0, 0], [0.6, 100, "expo.out"], [2, 100]]          keyframes [seconds, value, ease?]; the ease shapes the move INTO that key
//! {"from": 0, "to": 100, "at": 1, "dur": 0.6, "ease": "back.out(1.4)"}     a GSAP-style tween
//! {"keys": [[0, 0], [1, 360]], "loop": "cycle"}       keys that repeat: cycle | pingpong | offset
//! {"value": [[0, 0], [1, 100]], "expr": "value + wiggle(3, 5)"}             keys plus an expression on top
//! ```
//!
//! Before the first key a property holds its first value, after the last it holds its last (unless it loops).

mod colour;
mod key;
mod looping;
mod parse;
mod parse_keys;
mod parse_side;
mod shape;
mod spatial;
#[cfg(test)]
mod testkit;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_forms;
#[cfg(test)]
mod tests_spatial;
#[cfg(test)]
mod tests_temporal;
mod track;
mod value;
mod vector;
mod vector_parse;

use anyhow::Result;

use super::expr::{Arg, Env, Expr};
pub use colour::{parse_color_static, ColorProp, Rgba};
pub use key::{Handle, Key, Point, Rove, Side, Tangents};
pub use looping::Loop;
pub use parse_keys::KEY_FIELDS;
pub use shape::{AUTO_INFLUENCE, LINEAR_INFLUENCE};
pub use track::{Options, Track};
pub use value::{Kind, Value};
pub use vector::VecProp;

/// A number that can animate: a scalar track, and an expression applied on top of it.
#[derive(Debug, Clone)]
pub struct Prop {
    track: Track,
    expr: Option<Expr>,
}

impl Prop {
    pub fn constant(v: f64) -> Prop {
        Prop { track: Track::constant(Value::F(v)), expr: None }
    }

    pub fn is_constant(&self) -> bool {
        self.track.keys().len() == 1 && self.expr.is_none()
    }

    pub fn keys(&self) -> &[Key] {
        self.track.keys()
    }

    pub fn expr(&self) -> Option<&Expr> {
        self.expr.as_ref()
    }

    /// Plain keys: no expression and no loop, so the value is exactly what `base` says at any time.
    pub fn is_plain(&self) -> bool {
        self.expr.is_none() && self.track.looping() == Loop::None
    }

    /// The keyframed value at `t`, before any expression.
    pub fn base(&self, t: f64) -> f64 {
        self.track.sample(t).parts().v[0]
    }

    /// The value at `t` with the expression applied. `env` supplies `time`, `index`, and the host functions.
    pub fn at(&self, t: f64, env: &dyn Env) -> Result<f64> {
        let base = self.base(t);
        match &self.expr {
            None => Ok(base),
            Some(e) => e.eval(&WithValue { inner: env, value: base }),
        }
    }
}

struct WithValue<'a> {
    inner: &'a dyn Env,
    value: f64,
}

impl Env for WithValue<'_> {
    fn var(&self, name: &str) -> Option<f64> {
        if name == "value" { Some(self.value) } else { self.inner.var(name) }
    }
    fn call(&self, name: &str, args: &[Arg]) -> Option<Result<f64>> {
        self.inner.call(name, args)
    }
    fn salt(&self) -> u32 {
        self.inner.salt()
    }
}
