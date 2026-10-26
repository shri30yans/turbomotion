//! The types a property can have, and the value of each at an instant.

use anyhow::{anyhow, Result};
use serde_json::Value as Json;

use super::{colour::parse_color_static, parse::num};

/// What a property holds. Numbers (a scalar, a vector, a colour) change smoothly between keys; a switch and a piece
/// of text can only change at a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Scalar,
    Vec2,
    Vec3,
    /// Channels 0..255 and alpha 0..1, as written: `[r, g, b, a]`.
    Colour,
    Bool,
    Text,
}

/// The value of a property at one instant.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    F(f64),
    V2([f64; 2]),
    V3([f64; 3]),
    C([f64; 4]),
    B(bool),
    Text(String),
}

/// The numbers of a value that changes smoothly, one per dimension (the first `n` of four).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parts {
    pub n: usize,
    pub v: [f64; 4],
}

impl Kind {
    /// How many numbers the kind has; none for a switch or text.
    pub fn dims(self) -> usize {
        match self {
            Kind::Scalar => 1,
            Kind::Vec2 => 2,
            Kind::Vec3 => 3,
            Kind::Colour => 4,
            Kind::Bool | Kind::Text => 0,
        }
    }

    /// Whether values of this kind change smoothly between keys.
    pub fn interpolates(self) -> bool {
        self.dims() > 0
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Scalar => "a number",
            Kind::Vec2 => "a vector of 2 numbers",
            Kind::Vec3 => "a vector of 3 numbers",
            Kind::Colour => "a colour",
            Kind::Bool => "true or false",
            Kind::Text => "text",
        }
    }

    /// `json` as a value of this kind.
    pub fn read(self, json: &Json) -> Result<Value> {
        let mismatch = || anyhow!("expected {}, found {json}", self.name());
        Ok(match self {
            Kind::Scalar => Value::F(num("a key's value", json)?),
            Kind::Vec2 => Value::V2(numbers(json).and_then(|v| v.try_into().ok()).ok_or_else(mismatch)?),
            Kind::Vec3 => Value::V3(numbers(json).and_then(|v| v.try_into().ok()).ok_or_else(mismatch)?),
            Kind::Colour => Value::C(parse_color_static(json)?),
            Kind::Bool => Value::B(json.as_bool().ok_or_else(mismatch)?),
            Kind::Text => Value::Text(json.as_str().ok_or_else(mismatch)?.to_string()),
        })
    }
}

fn numbers(json: &Json) -> Option<Vec<f64>> {
    json.as_array()?.iter().map(Json::as_f64).collect()
}

impl Value {
    pub fn kind(&self) -> Kind {
        match self {
            Value::F(_) => Kind::Scalar,
            Value::V2(_) => Kind::Vec2,
            Value::V3(_) => Kind::Vec3,
            Value::C(_) => Kind::Colour,
            Value::B(_) => Kind::Bool,
            Value::Text(_) => Kind::Text,
        }
    }

    /// The numbers of a value that changes smoothly; none for a switch or text.
    pub fn parts(&self) -> Parts {
        let mut v = [0.0; 4];
        let n = match self {
            Value::F(x) => {
                v[0] = *x;
                1
            }
            Value::V2(a) => {
                v[..2].copy_from_slice(a);
                2
            }
            Value::V3(a) => {
                v[..3].copy_from_slice(a);
                3
            }
            Value::C(a) => {
                v = *a;
                4
            }
            Value::B(_) | Value::Text(_) => 0,
        };
        Parts { n, v }
    }

    /// A value of the same kind with these numbers (the first `dims` of them); a switch or text has none and stays.
    pub fn with_parts(&self, v: [f64; 4]) -> Value {
        match self {
            Value::F(_) => Value::F(v[0]),
            Value::V2(_) => Value::V2([v[0], v[1]]),
            Value::V3(_) => Value::V3([v[0], v[1], v[2]]),
            Value::C(_) => Value::C(v),
            Value::B(_) | Value::Text(_) => self.clone(),
        }
    }

    /// The place a vector stands for: `[x, y, 0]` for a 2D one, `[x, y, z]` for a 3D one.
    pub fn point(&self) -> [f64; 3] {
        let p = self.parts();
        [p.v[0], p.v[1], p.v[2]]
    }

    /// The numbers of this value as a list (empty for a switch or text).
    pub fn to_vec(&self) -> Vec<f64> {
        let p = self.parts();
        p.v[..p.n].to_vec()
    }
}
