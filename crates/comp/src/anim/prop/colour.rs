//! Colours that animate: channels are 0..255, alpha 0..1.

use anyhow::{anyhow, bail, ensure, Result};
use serde_json::Value;

use super::{
    parse::{loop_of, num},
    parse_keys::Reader,
    track::{Options, Track},
    value::Kind,
    Prop,
};
use crate::anim::expr::Env;

/// An RGBA colour that can animate: channels are 0..255, alpha 0..1.
#[derive(Debug, Clone)]
pub enum ColorProp {
    /// One property per channel, each with its own keys and expression.
    Channels { r: Prop, g: Prop, b: Prop, a: Prop },
    /// One track for the whole colour, keys written as objects with handles. The channels are interpolated as written
    /// (see `parity/specs/property_model.md` for why not in linear light).
    Joined(Track),
}

pub type Rgba = [f32; 4];

pub fn parse_color_static(v: &Value) -> Result<[f64; 4]> {
    match v {
        Value::String(s) => hex(s),
        Value::Array(a) if a.len() == 3 || a.len() == 4 => {
            let n: Vec<f64> = a.iter().map(|c| num("a colour channel", c)).collect::<Result<_>>()?;
            Ok([n[0], n[1], n[2], n.get(3).copied().unwrap_or(1.0)])
        }
        other => bail!("a colour is \"#rrggbb\", \"#rrggbbaa\", a name like \"white\", or [r, g, b, a?] (0-255, alpha 0-1); found {other}"),
    }
}

/// The key list of a colour animation, in either spelling: `[[t, c], ..]` or `{"keys": [[t, c], ..]}`.
fn colour_keys(v: &Value) -> Option<&Vec<Value>> {
    let list = match v {
        Value::Array(l) => l,
        Value::Object(o) => o.get("keys")?.as_array()?,
        _ => return None,
    };
    list.first().is_some_and(|k| k.is_array() || k.is_object()).then_some(list)
}

fn hex(s: &str) -> Result<[f64; 4]> {
    let named = match s {
        "white" => Some([255.0, 255.0, 255.0, 1.0]),
        "black" => Some([0.0, 0.0, 0.0, 1.0]),
        "transparent" | "none" => Some([0.0, 0.0, 0.0, 0.0]),
        "red" => Some([232.0, 64.0, 47.0, 1.0]),
        "green" => Some([62.0, 182.0, 150.0, 1.0]),
        "blue" => Some([60.0, 120.0, 255.0, 1.0]),
        "yellow" => Some([255.0, 214.0, 10.0, 1.0]),
        "gold" => Some([222.0, 176.0, 82.0, 1.0]),
        _ => None,
    };
    if let Some(c) = named {
        return Ok(c);
    }
    let h = s.strip_prefix('#').ok_or_else(|| anyhow!("colour {s:?} should start with # (like \"#ff8800\") or be a name like \"white\""))?;
    let d = |i: usize| u8::from_str_radix(&h[i..i + 1].repeat(2), 16);
    let b = |i: usize| u8::from_str_radix(&h[i..i + 2], 16);
    let bad = |_| anyhow!("colour {s:?} has non-hex digits");
    let c = match h.len() {
        3 => [d(0).map_err(bad)?, d(1).map_err(bad)?, d(2).map_err(bad)?, 255],
        4 => [d(0).map_err(bad)?, d(1).map_err(bad)?, d(2).map_err(bad)?, d(3).map_err(bad)?],
        6 => [b(0).map_err(bad)?, b(2).map_err(bad)?, b(4).map_err(bad)?, 255],
        8 => [b(0).map_err(bad)?, b(2).map_err(bad)?, b(4).map_err(bad)?, b(6).map_err(bad)?],
        _ => bail!("colour {s:?} should have 3, 4, 6 or 8 hex digits"),
    };
    Ok([f64::from(c[0]), f64::from(c[1]), f64::from(c[2]), f64::from(c[3]) / 255.0])
}

impl ColorProp {
    pub fn constant(c: [f64; 4]) -> ColorProp {
        ColorProp::Channels { r: Prop::constant(c[0]), g: Prop::constant(c[1]), b: Prop::constant(c[2]), a: Prop::constant(c[3]) }
    }

    /// A colour, a tween `{"from": "#fff", "to": "#f00", "at", "dur", "ease"}`, keys `[[t, "#fff", ease?], ..]`, or
    /// `{"keys": [[t, "#fff", ease?], ..], "loop": "cycle"}`.
    pub fn parse(v: &Value) -> Result<ColorProp> {
        let looping = v.get("loop").cloned();
        let split = |keys: Vec<(f64, [f64; 4], Value)>| -> Result<ColorProp> {
            let mk = |c: usize| -> Result<Prop> {
                let list: Vec<Value> = keys
                    .iter()
                    .map(|(t, col, ease)| {
                        let mut k = vec![Value::from(*t), Value::from(col[c])];
                        if !ease.is_null() {
                            k.push(ease.clone());
                        }
                        Value::Array(k)
                    })
                    .collect();
                match &looping {
                    None => Prop::parse(&Value::Array(list)),
                    Some(l) => Prop::parse(&serde_json::json!({"keys": list, "loop": l})),
                }
            };
            Ok(ColorProp::Channels { r: mk(0)?, g: mk(1)?, b: mk(2)?, a: mk(3)? })
        };
        match v {
            Value::Object(o) if o.contains_key("channels") => {
                ensure!(o.len() == 1, "colour channels cannot be combined with other fields");
                let channels = o["channels"].as_array().ok_or_else(|| anyhow!("colour channels must be [r,g,b,a] properties"))?;
                ensure!(channels.len() == 4, "colour channels must have four properties: [r,g,b,a]");
                Ok(ColorProp::Channels { r: Prop::parse(&channels[0])?, g: Prop::parse(&channels[1])?, b: Prop::parse(&channels[2])?, a: Prop::parse(&channels[3])? })
            }
            Value::Object(o) if o.contains_key("from") => {
                let from = parse_color_static(&o["from"])?;
                let to = parse_color_static(o.get("to").ok_or_else(|| anyhow!("a colour tween needs \"to\""))?)?;
                let at = o.get("at").map_or(Ok(0.0), |v| num("\"at\"", v))? + o.get("delay").map_or(Ok(0.0), |v| num("\"delay\"", v))?;
                let dur = o.get("dur").map_or(Ok(0.5), |v| num("\"dur\"", v))?;
                let ease = o.get("ease").cloned().unwrap_or(Value::Null);
                split(vec![(at, from, Value::Null), (at + dur.max(1e-6), to, ease)])
            }
            Value::Array(_) | Value::Object(_) if colour_keys(v).is_some_and(|l| l.iter().any(Value::is_object)) => {
                let keys = match v {
                    Value::Object(o) => o.get("keys").ok_or_else(|| anyhow!("a colour with keys needs \"keys\""))?,
                    keys => keys,
                };
                let reader = Reader { kind: Kind::Colour, path: None };
                let looping = v.as_object().map_or(Ok(Default::default()), loop_of)?;
                Ok(ColorProp::Joined(Track::new(Kind::Colour, reader.keys(keys)?, Options { looping, spatial: false })?))
            }
            Value::Array(_) | Value::Object(_) if colour_keys(v).is_some() => {
                let list = colour_keys(v).expect("checked in the guard");
                let keys = list
                    .iter()
                    .map(|k| {
                        let k = k.as_array().ok_or_else(|| anyhow!("colour keys are [seconds, colour, ease?]"))?;
                        ensure!(k.len() >= 2, "colour keys are [seconds, colour, ease?]");
                        Ok((num("a colour key's time", &k[0])?, parse_color_static(&k[1])?, k.get(2).cloned().unwrap_or(Value::Null)))
                    })
                    .collect::<Result<Vec<_>>>()?;
                split(keys)
            }
            Value::Object(_) => bail!("a colour animation is {{\"from\": c, \"to\": c, \"at\", \"dur\", \"ease\"}} or {{\"keys\": [[seconds, colour, ease?], ..]}}; found {v}"),
            other => Ok(ColorProp::constant(parse_color_static(other)?)),
        }
    }

    pub fn at(&self, t: f64, env: &dyn Env) -> Result<Rgba> {
        let [r, g, b, a] = match self {
            ColorProp::Channels { r, g, b, a } => [r.at(t, env)?, g.at(t, env)?, b.at(t, env)?, a.at(t, env)?],
            ColorProp::Joined(track) => track.sample(t).parts().v,
        };
        Ok([r.clamp(0.0, 255.0) as f32, g.clamp(0.0, 255.0) as f32, b.clamp(0.0, 255.0) as f32, a.clamp(0.0, 1.0) as f32])
    }
}
